#!/bin/sh
# Copies the working tree into the VM (no build output, no git, no VM state).
HERE=$(cd "$(dirname "$0")" && pwd); ROOT=$(cd "$HERE/../.." && pwd)
exec rsync -a --delete --exclude target --exclude /dist --exclude .git --exclude ci/vm/state --exclude ci/spike/out \
  -e "ssh -p 2222 -i $HERE/state/key -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR" \
  "$ROOT/" dev@127.0.0.1:PatineDE/
