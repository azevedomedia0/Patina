#!/bin/sh
# Drives a running two-window DRM session through a varied sequence of interactions from the host,
# for testing rendering. Needs the VM up and `drm-two.sh` running.
Q="$(dirname "$0")/qmp.py"
step() { sleep "${2:-0.7}"; }
$Q move 0.6297 0.0975; step                       # hover the front window's close button
$Q drag 0.35 0.0975 0.55 0.40; step 1.2           # move the front window right and down
$Q drag 0.55 0.40 0.20 0.20; step 1.2             # and back across the other window
$Q move 0.086 0.375; step; $Q click; step         # raise the window behind
$Q move 0.30 0.40; step; $Q click; step           # click into a terminal
for k in e c h o spc h i ret; do $Q key $k; done; step 1.2
$Q move 0.5836 0.0688; step; $Q click; step 1.5   # maximise
$Q move 0.469 0.0764; step; $Q click; $Q click; step 1.5   # double-click title to restore
$Q combo meta_l shift 2; step 1.5                 # move the window to workspace 2
$Q combo meta_l 1; step 1.5                       # back to workspace 1
$Q combo meta_l 2; step 1.5
$Q combo meta_l 1; step 1.5
$Q move 0.9 0.9; step 1
