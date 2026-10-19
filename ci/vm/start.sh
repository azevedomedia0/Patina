#!/bin/sh
# Starts the Patina build VM in the background. ssh: ci/vm/ssh.sh (host port 2222).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd); S=$HERE/state
QSHARE=$(dirname "$(dirname "$(realpath "$(command -v qemu-system-aarch64)")")")/share/qemu
if [ -f "$S/qemu.pid" ] && kill -0 "$(cat "$S/qemu.pid")" 2>/dev/null; then echo "already running"; exit 0; fi
qemu-system-aarch64 \
  -accel hvf -M virt,highmem=on -cpu host -smp 6 -m 8G \
  -drive if=pflash,format=raw,file="$QSHARE/edk2-aarch64-code.fd",readonly=on \
  -drive if=pflash,format=raw,file="$S/vars.fd" \
  -drive if=virtio,format=qcow2,file="$S/disk.qcow2" \
  -drive if=virtio,format=raw,file="$S/seed.iso",readonly=on \
  -netdev user,id=n0,hostfwd=tcp:127.0.0.1:2222-:22 -device virtio-net-pci,netdev=n0 \
  -device virtio-gpu-pci -device qemu-xhci -device usb-kbd -device usb-tablet \
  -display none -serial file:"$S/serial.log" \
  -qmp unix:"$S/qmp.sock",server,nowait \
  -pidfile "$S/qemu.pid" -daemonize
echo "started; pid $(cat "$S/qemu.pid")"
