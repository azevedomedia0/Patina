#!/bin/sh
# DRM session with two terminals titled "Terminal" and "File Bin", to test focus-dependent UI.
exec sh "$HOME/PatineDE/ci/vm/guest/drm-up.sh" 'foot -T Terminal' 'foot -T "File Bin"'
