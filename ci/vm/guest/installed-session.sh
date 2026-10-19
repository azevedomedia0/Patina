#!/bin/sh
# Runs inside the VM. Starts a session with the INSTALLED package (/usr/bin/patina-session),
# on the VM's display through seatd, like a login from a text console would.
export XDG_RUNTIME_DIR=/tmp/xdg-patina-installed
for p in '[t]arget/release/patina-' '[u]sr/bin/patina-' '^patina-' '^dbus-run-session' '^foot' '^weston-simple-shm' '^sway'; do pkill -u "$(id -u)" -f "$p" >/dev/null 2>&1; done
sleep 1
rm -rf "$XDG_RUNTIME_DIR" "$HOME/.local/state/patina"; mkdir -p "$XDG_RUNTIME_DIR"; chmod 700 "$XDG_RUNTIME_DIR"
unset WAYLAND_DISPLAY DISPLAY DBUS_SESSION_BUS_ADDRESS
export LIBSEAT_BACKEND=seatd
setsid /usr/bin/patina-session 'foot -T Terminal' >/dev/null 2>&1 &
sleep 8
echo "--- processes"; pgrep -af -u "$(id -u)" 'patina|foot|dbus-run' | cut -c1-90
echo "--- session.log"; sed 's/\x1b\[[0-9;]*m//g' "$HOME/.local/state/patina/session.log" | grep -v -E "EGL|GL_|smithay::input|^info:|^warn:" | cut -c1-200 | head -20
