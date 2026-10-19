#!/bin/sh
# Runs inside the (arm64) VM. Smoke test of the amd64 package: installs it into a Debian 12
# amd64 chroot (run through qemu-user) and starts its compositor and panel nested, as a window
# inside the arm64 Patina session that drm-up.sh started. Slow (everything x86 is emulated),
# but it shows the amd64 binaries start and draw.
set -eu
DEB=$(ls "$HOME"/PatineDE/dist/bookworm-amd64/patina-desktop_*.deb)
CHROOT=/srv/amd64-smoke
HOST_RUNTIME=/tmp/xdg-patina-drm
[ -S "$HOST_RUNTIME/wayland-1" ] || { echo "start a session first (drm-up.sh)"; exit 1; }

cleanup() {
  for m in "$CHROOT/run/host" "$CHROOT/proc" "$CHROOT/dev"; do
    mountpoint -q "$m" && sudo umount "$m"
  done
  if grep -q " $CHROOT" /proc/self/mountinfo; then
    echo "warning: $CHROOT still has mounts; not deleting it" >&2
    return 0
  fi
  [ "${KEEP:-0}" = 1 ] || sudo rm -rf --one-file-system "$CHROOT"
}
trap cleanup EXIT

command -v qemu-x86_64-static >/dev/null || sudo apt-get install -y -q qemu-user-static binfmt-support
if [ ! -d "$CHROOT/usr" ]; then
  sudo debootstrap --variant=minbase --arch=amd64 bookworm "$CHROOT" http://deb.debian.org/debian >/dev/null
fi
sudo cp /etc/resolv.conf "$CHROOT/etc/resolv.conf"
sudo cp "$DEB" "$CHROOT/tmp/patina.deb"
sudo mkdir -p "$CHROOT/run/host"
mountpoint -q "$CHROOT/proc" || sudo mount -t proc proc "$CHROOT/proc"
mountpoint -q "$CHROOT/dev" || sudo mount --bind /dev "$CHROOT/dev"
# The host session's Wayland socket, so the nested compositor can open its window there.
sudo mount --bind "$HOST_RUNTIME" "$CHROOT/run/host"

sudo chroot "$CHROOT" /bin/sh -eu -c '
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -q >/dev/null
  apt-get install -y -q /tmp/patina.deb foot >/dev/null 2>&1
  uname -m
  export XDG_RUNTIME_DIR=/run/host WAYLAND_DISPLAY=wayland-1 WGPU_BACKEND=gl RUST_LOG=info
  timeout 90 patina-compositor --per-output patina-panel "foot -T amd64" >/tmp/nested.log 2>&1 || true
  sed "s/\x1b\[[0-9;]*m//g" /tmp/nested.log | grep -E "listening|started|panic|ERROR" | cut -c1-140 | head
'
