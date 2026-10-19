#!/bin/sh
# Runs inside the VM as the dev user. Builds the .deb in a pristine chroot of the given release, so
# that anything missing from debian/control's Build-Depends fails the build instead of being masked
# by packages already installed in the VM. The package is copied to dist/<release>/.
#
# Usage: chroot-build.sh [release]   bookworm (default), trixie, jammy or noble
set -eu
RELEASE=${1:-bookworm}
CHROOT=/srv/chroot-$RELEASE
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
  sudo debootstrap --variant=minbase --components="$COMPONENTS" \
    --include=ca-certificates,curl,rsync "$RELEASE" "$CHROOT" "$MIRROR"
fi

# The VM's disk image lives on the Mac and can only grow. So the toolchain and the crate cache are
# shared with the VM instead of copied into every chroot, and the chroot is deleted afterwards
# (KEEP=1 keeps it). Delete with --one-file-system, so a bind mount can never be followed.
cleanup() {
  for m in root/.cargo/registry root/.rustup proc; do
    mountpoint -q "$CHROOT/$m" && sudo umount "$CHROOT/$m"
  done
  if [ "${KEEP:-0}" != 1 ]; then
    sudo rm -rf --one-file-system "$CHROOT"
  fi
}
trap cleanup EXIT

sudo mkdir -p "$CHROOT/build"
sudo rsync -a --delete \
  --exclude /target --exclude .git --exclude /dist --exclude /ci/vm/state --exclude /ci/spike/out \
  "$SRC/" "$CHROOT/build/PatineDE/"
sudo cp /etc/resolv.conf "$CHROOT/etc/resolv.conf"
mountpoint -q "$CHROOT/proc" || sudo mount -t proc proc "$CHROOT/proc"
mkdir -p "$HOME/.cargo/registry" "$HOME/.rustup"
sudo mkdir -p "$CHROOT/root/.cargo/registry" "$CHROOT/root/.rustup"
mountpoint -q "$CHROOT/root/.cargo/registry" || sudo mount --bind "$HOME/.cargo/registry" "$CHROOT/root/.cargo/registry"
mountpoint -q "$CHROOT/root/.rustup" || sudo mount --bind "$HOME/.rustup" "$CHROOT/root/.rustup"

sudo chroot "$CHROOT" /bin/sh -c '
  export DEBIAN_FRONTEND=noninteractive HOME=/root
  apt-get update -q >/dev/null
  cd /build/PatineDE
  packaging/debian/build.sh --bootstrap-rust --install-build-deps
'
mkdir -p "$OUT/$RELEASE"
sudo cp "$CHROOT"/build/PatineDE/dist/patina-desktop_* "$OUT/$RELEASE/"
sudo chown -R "$(id -u):$(id -g)" "$OUT"
ls -l "$OUT/$RELEASE"
