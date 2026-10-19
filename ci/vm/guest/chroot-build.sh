#!/bin/sh
# Runs inside the VM as the dev user. Builds the .deb in a pristine chroot of the given release, so
# that anything missing from debian/control's Build-Depends fails the build instead of being masked
# by packages already installed in the VM. The package is copied to dist/<release>/.
#
# Usage: chroot-build.sh [release] [host-arch]   bookworm (default), trixie, jammy or noble;
#        host-arch cross-builds for another architecture (amd64; Debian releases only, since
#        Ubuntu keeps amd64 and arm64 on different mirrors)
set -eu
RELEASE=${1:-bookworm}
HOST_ARCH=${2:-}
CHROOT=/srv/chroot-$RELEASE${HOST_ARCH:+-$HOST_ARCH}
# SRC is the tree to build (default: the working tree); OUT receives the packages.
SRC=${SRC:-$HOME/PatineDE}
OUT=${OUT:-$HOME/PatineDE/dist}

case "$RELEASE" in
  bookworm | trixie)
    MIRROR=http://deb.debian.org/debian; COMPONENTS=main ;;
  jammy | noble)
    # arm64 Ubuntu lives on the ports mirror, and several dependencies are in universe.
    MIRROR=http://ports.ubuntu.com/ubuntu-ports; COMPONENTS=main,universe ;;
  *) echo "unknown release: $RELEASE" >&2; exit 2 ;;
esac

if [ ! -d "$CHROOT/usr" ]; then
  sudo apt-get install -y -q debootstrap ubuntu-keyring debian-archive-keyring
  # Debian 12's debootstrap predates Ubuntu 24.04; its script is the same as for older releases.
  [ -e /usr/share/debootstrap/scripts/noble ] || sudo ln -s gutsy /usr/share/debootstrap/scripts/noble
  sudo debootstrap --variant=minbase --components="$COMPONENTS" \
    --include=ca-certificates,curl,rsync "$RELEASE" "$CHROOT" "$MIRROR"
fi

# The VM's disk image lives on the Mac and can only grow, so the chroot is deleted afterwards
# (KEEP=1 keeps it). Nothing from the VM is bind-mounted into it: a bind mount of a directory on the
# same disk shares its device id, so `rm --one-file-system` would happily delete through it (this
# once emptied the VM's cargo and rustup directories). Only /proc is mounted, which is a different
# filesystem and is unmounted before anything is deleted.
cleanup() {
  if mountpoint -q "$CHROOT/proc"; then
    sudo umount "$CHROOT/proc" || { echo "warning: cannot unmount $CHROOT/proc; not deleting" >&2; return 0; }
  fi
  if [ "${KEEP:-0}" = 1 ]; then return 0; fi
  if grep -q " $CHROOT" /proc/self/mountinfo; then
    echo "warning: $CHROOT still has mounts; not deleting it" >&2
    return 0
  fi
  sudo rm -rf --one-file-system "$CHROOT" || true
}
trap cleanup EXIT

sudo mkdir -p "$CHROOT/build"
sudo rsync -a --delete \
  --exclude /target --exclude .git --exclude /dist --exclude /ci/vm/state --exclude /ci/spike/out \
  "$SRC/" "$CHROOT/build/PatineDE/"
sudo cp /etc/resolv.conf "$CHROOT/etc/resolv.conf"
mountpoint -q "$CHROOT/proc" || sudo mount -t proc proc "$CHROOT/proc"

# HOST_ARCH is passed through the environment: the script below is single-quoted.
sudo env HOST_ARCH="$HOST_ARCH" chroot "$CHROOT" /bin/sh -c '
  set -e
  export DEBIAN_FRONTEND=noninteractive HOME=/root
  if [ -n "$HOST_ARCH" ]; then dpkg --add-architecture "$HOST_ARCH"; fi
  apt-get update -q >/dev/null
  cd /build/PatineDE
  packaging/debian/build.sh --bootstrap-rust --install-build-deps ${HOST_ARCH:+--host-arch "$HOST_ARCH"}
'
DEST="$OUT/$RELEASE${HOST_ARCH:+-$HOST_ARCH}"
mkdir -p "$DEST"
sudo cp "$CHROOT"/build/PatineDE/dist/patina-desktop_* "$DEST/"
sudo chown -R "$(id -u):$(id -g)" "$OUT"
ls -l "$DEST"
