#!/bin/sh
# Runs inside the VM: screenshot sway's output to /tmp/shot-NAME.png
export XDG_RUNTIME_DIR=/tmp/xdg-patina
export WAYLAND_DISPLAY=$(ls "$XDG_RUNTIME_DIR" | grep '^wayland-[0-9]*$' | head -1)
grim "/tmp/shot-${1:-x}.png" && echo "/tmp/shot-${1:-x}.png"
