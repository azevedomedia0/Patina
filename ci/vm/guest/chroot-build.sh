#!/bin/sh
# Runs inside the VM as the dev user. Builds the .deb in a pristine Debian 12 chroot, so that
# anything missing from debian/control's Build-Depends fails the build instead of being masked
# by packages already installed in the VM.
#
# Usage: chroot-build.sh [release]      (default: bookworm)
set -eu
RELEASE=${1:-bookworm}
CHROOT=/srv/chroot-$RELEASE
SRC=$HOME/PatineDE

if [ ! -d "$CHROOT/usr" ]; then
  sudo apt-get install -y -q debootstrap
  sudo debootstrap --variant=minbase --include=ca-certificates,curl,rsync "$RELEASE" "$CHROOT" http://deb.debian.org/debian
fi

sudo mkdir -p "$CHROOT/build"
sudo rsync -a --delete \
  --exclude /target --exclude .git --exclude /dist --exclude /ci/vm/state --exclude /ci/spike/out \
  "$SRC/" "$CHROOT/build/PatineDE/"
sudo cp /etc/resolv.conf "$CHROOT/etc/resolv.conf"
mountpoint -q "$CHROOT/proc" || sudo mount -t proc proc "$CHROOT/proc"

sudo chroot "$CHROOT" /bin/sh -c '
  export DEBIAN_FRONTEND=noninteractive HOME=/root
  cd /build/PatineDE
  packaging/debian/build.sh --bootstrap-rust --install-build-deps
'
mkdir -p "$SRC/dist"
sudo cp "$CHROOT"/build/PatineDE/dist/patina-desktop_* "$SRC/dist/"
sudo chown "$(id -u):$(id -g)" "$SRC"/dist/*
ls -l "$SRC"/dist
