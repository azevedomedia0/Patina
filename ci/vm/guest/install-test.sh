#!/bin/sh
# Runs inside the VM as the dev user. Installs the .deb for a release into a bare chroot of that
# release (minbase, nothing else), so that missing or unresolvable runtime dependencies show up,
# then checks the installed files, that no binary has an unresolved library, and that purging
# removes everything.
#
# Usage: install-test.sh release [deb]   deb defaults to ~/PatineDE/dist/<release>/*.deb
set -eu
RELEASE=${1:?release}
DEB=${2:-$(ls "$HOME"/PatineDE/dist/"$RELEASE"/patina-desktop_*.deb)}
CHROOT=/srv/bare-$RELEASE

case "$RELEASE" in
  bookworm | trixie) MIRROR=http://deb.debian.org/debian; COMPONENTS=main ;;
  jammy | noble) MIRROR=http://ports.ubuntu.com/ubuntu-ports; COMPONENTS=main,universe ;;
  *) echo "unknown release: $RELEASE" >&2; exit 2 ;;
esac

cleanup() {
  if mountpoint -q "$CHROOT/proc"; then
    sudo umount "$CHROOT/proc" || { echo "warning: cannot unmount; not deleting $CHROOT" >&2; return 0; }
  fi
  if grep -q " $CHROOT" /proc/self/mountinfo; then return 0; fi
  sudo rm -rf --one-file-system "$CHROOT" || true
}
trap cleanup EXIT

if [ ! -d "$CHROOT/usr" ]; then
  [ -e /usr/share/debootstrap/scripts/noble ] || sudo ln -s gutsy /usr/share/debootstrap/scripts/noble
  sudo debootstrap --variant=minbase --components="$COMPONENTS" "$RELEASE" "$CHROOT" "$MIRROR"
fi
sudo cp /etc/resolv.conf "$CHROOT/etc/resolv.conf"
sudo cp "$DEB" "$CHROOT/tmp/patina.deb"
mountpoint -q "$CHROOT/proc" || sudo mount -t proc proc "$CHROOT/proc"

sudo chroot "$CHROOT" /bin/sh -eu -c '
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -q >/dev/null
  apt-get install -y -q /tmp/patina.deb >/tmp/install.log 2>&1 || { tail -20 /tmp/install.log; exit 1; }
  echo "installed: $(dpkg-query -W -f="\${Version}" patina-desktop)"
  for f in /usr/bin/patina-compositor /usr/bin/patina-panel /usr/bin/patina-session \
           /usr/share/wayland-sessions/patina.desktop /usr/share/man/man1/patina-compositor.1.gz; do
    [ -e "$f" ] || { echo "MISSING $f"; exit 1; }
  done
  for b in /usr/bin/patina-compositor /usr/bin/patina-panel; do
    if ldd "$b" | grep "not found"; then echo "UNRESOLVED libraries in $b"; exit 1; fi
  done
  echo "libraries resolve; depends pulled in $(grep -c "^Setting up" /tmp/install.log) packages"
  apt-get purge -y -q patina-desktop >/dev/null 2>&1
  apt-get autoremove -y -q >/dev/null 2>&1
  left=$(dpkg -L patina-desktop 2>/dev/null | head -1 || true)
  ls /usr/bin/patina-* /usr/share/wayland-sessions/patina.desktop 2>/dev/null && { echo "FILES LEFT AFTER PURGE"; exit 1; }
  echo "purge clean"
'
echo "$RELEASE: install test passed"
