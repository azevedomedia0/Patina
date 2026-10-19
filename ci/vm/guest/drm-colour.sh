#!/bin/sh
# DRM session with a brightly coloured window and a terminal, to see what glass does to sharp
# content behind it.
exec sh "$HOME/PatineDE/ci/vm/guest/drm-up.sh" weston-simple-shm 'foot -T Terminal'
