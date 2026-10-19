#!/bin/sh
# ssh into the build VM. With no args opens a shell; otherwise runs the command.
HERE=$(cd "$(dirname "$0")" && pwd)
exec ssh -p 2222 -i "$HERE/state/key" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ConnectTimeout=5 dev@127.0.0.1 "$@"
