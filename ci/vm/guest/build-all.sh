#!/bin/sh
# Builds the package for every given release in turn, logging each to /tmp/build-<release>.log.
# It builds a frozen copy of the working tree taken now, so editing the tree meanwhile is safe.
SNAP=$HOME/PatineDE-snapshot
rsync -a --delete --exclude /target --exclude /dist --exclude /ci/vm/state "$HOME/PatineDE/" "$SNAP/"
export SRC=$SNAP
for r in "$@"; do
  echo "=== $r: $(date +%T)"
  if sh "$HOME/PatineDE/ci/vm/guest/chroot-build.sh" "$r" >"/tmp/build-$r.log" 2>&1; then
    echo "$r: built"
  else
    echo "$r: FAILED (see /tmp/build-$r.log)"
  fi
done
echo "=== done: $(date +%T)"
