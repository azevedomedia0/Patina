#!/bin/sh
# One-time VM setup: disk from the Debian 12 arm64 cloud image, ssh key, cloud-init seed.
# Usage: ci/vm/setup.sh /path/to/debian-12-genericcloud-arm64.qcow2
set -eu
BASE=${1:?path to debian-12-genericcloud-arm64.qcow2}
HERE=$(cd "$(dirname "$0")" && pwd); S=$HERE/state; mkdir -p "$S"
[ -f "$S/key" ] || ssh-keygen -q -t ed25519 -N '' -C patina-vm -f "$S/key"
if [ ! -f "$S/disk.qcow2" ]; then
  cp "$BASE" "$S/disk.qcow2"
  qemu-img resize "$S/disk.qcow2" 40G
fi
QSHARE=$(dirname "$(dirname "$(realpath "$(command -v qemu-system-aarch64)")")")/share/qemu
[ -f "$S/vars.fd" ] || cp "$QSHARE/edk2-arm-vars.fd" "$S/vars.fd"
rm -rf "$S/seed" "$S/seed.iso"; mkdir "$S/seed"
sed "s|@PUBKEY@|$(cat "$S/key.pub")|" "$HERE/user-data.tmpl" > "$S/seed/user-data"
printf 'instance-id: patina-vm-1\nlocal-hostname: patina-vm\n' > "$S/seed/meta-data"
hdiutil makehybrid -quiet -iso -joliet -default-volume-name cidata -o "$S/seed.iso" "$S/seed"
echo "setup ok: $S"
