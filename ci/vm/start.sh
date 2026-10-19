#!/bin/sh
# Starts the Patina build VM in the background. SECOND_GPU=1 adds a second virtio GPU with its
# own display (a two-monitor, two-GPU setup; it cannot be hot-plugged).
# Suspend cannot be tested here: nothing wakes the arm64 guest from s2idle (RTC, USB input and
# QMP system_wakeup all fail), so a suspended VM has to be reset. The two PCIe root ports (at fixed addresses, so
# the firmware's boot entry still finds the disk) take hot-plugged display adapters: qmp.py plug.
# ssh: ci/vm/ssh.sh (host port 2222).
set -eu
EXTRA=""
# OUTPUTS=2 gives the first GPU two connectors; the second starts disconnected and can be
# forced on and off in the guest (echo on|off > /sys/class/drm/card0-Virtual-2/status).
GPU0="virtio-gpu-pci,id=gpu0${OUTPUTS:+,max_outputs=$OUTPUTS}"
[ "${SECOND_GPU:-0}" = 1 ] && EXTRA="-device virtio-gpu-pci,id=gpu1,bus=hp1"
# TOUCH=1 adds a touchscreen (driven with qmp.py tap and swipe).
[ "${TOUCH:-0}" = 1 ] && EXTRA="$EXTRA -device virtio-multitouch-pci,addr=0x13"
HERE=$(cd "$(dirname "$0")" && pwd); S=$HERE/state
QSHARE=$(dirname "$(dirname "$(realpath "$(command -v qemu-system-aarch64)")")")/share/qemu
if [ -f "$S/qemu.pid" ] && kill -0 "$(cat "$S/qemu.pid")" 2>/dev/null; then echo "already running"; exit 0; fi
qemu-system-aarch64 \
  -accel hvf -M virt,highmem=on -cpu host -smp 6 -m 8G \
  -drive if=pflash,format=raw,file="$QSHARE/edk2-aarch64-code.fd",readonly=on \
  -drive if=pflash,format=raw,file="$S/vars.fd" \
  -drive if=virtio,format=qcow2,file="$S/disk.qcow2",discard=unmap,detect-zeroes=unmap \
  -drive if=virtio,format=raw,file="$S/seed.iso",readonly=on \
  -netdev user,id=n0,hostfwd=tcp:127.0.0.1:2222-:22 -device virtio-net-pci,netdev=n0 \
  -device $GPU0 -device qemu-xhci \
  -device pcie-root-port,id=hp1,chassis=1,slot=1,addr=0x10 \
  -device pcie-root-port,id=hp2,chassis=2,slot=2,addr=0x11 $EXTRA -device usb-kbd -device usb-tablet \
  -display none -serial file:"$S/serial.log" \
  -qmp unix:"$S/qmp.sock",server,nowait \
  -pidfile "$S/qemu.pid" -daemonize
echo "started; pid $(cat "$S/qemu.pid")"
