#!/bin/sh
# Small, typical updates in a one-window DRM session: hovering the titlebar buttons, typing, and a
# short window nudge. For measuring what a partial repaint saves.
Q="$(dirname "$0")/qmp.py"
for x in 0.5836 0.6055 0.5617 0.6055 0.5836; do $Q move $x 0.0688; sleep 0.5; done   # hover the three buttons
$Q move 0.30 0.40; sleep 0.4; $Q click; sleep 0.4
for k in l s spc minus l ret e c h o spc h i ret; do $Q key $k; sleep 0.25; done    # type a few commands
for k in d a t e ret; do $Q key $k; sleep 0.25; done
$Q drag 0.35 0.0688 0.37 0.0988; sleep 1                                            # nudge the window
$Q move 0.9 0.9; sleep 0.8
