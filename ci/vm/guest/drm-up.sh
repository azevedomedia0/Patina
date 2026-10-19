#!/bin/sh
# Runs inside the VM. Starts patina-compositor on the real (virtual) display through DRM/KMS,
# via seatd, with the panel and the given clients (default: one foot terminal).
export XDG_RUNTIME_DIR=/tmp/xdg-patina-drm
# Process names are truncated to 15 characters, so match full command lines. The [x] bracket
# trick stops pkill matching its own shell.
for p in '[t]arget/release/patina-compositor' '[t]arget/release/patina-panel' '^patina-' '^dbus-run-session' '^foot' '^weston-simple-shm' '^sway'; do pkill -u "$(id -u)" -f "$p" >/dev/null 2>&1; done
sleep 1
rm -rf "$XDG_RUNTIME_DIR"; mkdir -p "$XDG_RUNTIME_DIR"; chmod 700 "$XDG_RUNTIME_DIR"
unset WAYLAND_DISPLAY DISPLAY
export LIBSEAT_BACKEND=seatd PATINA_BACKEND=drm
BIN=$HOME/PatineDE/target/release
[ $# -eq 0 ] && set -- 'foot -T Terminal'
setsid env RUST_LOG=${PATINA_LOG:-info,patina_compositor=debug} "$BIN/patina-compositor" "${PANEL_CMD:-WGPU_BACKEND=gl $BIN/patina-panel}" "$@" >/tmp/patina-drm.log 2>&1 &
sleep 8
echo "--- log"; sed 's/\x1b\[[0-9;]*m//g' /tmp/patina-drm.log | grep -v -E "EGL|GL_|smithay::input" | cut -c1-220 | head -30
pgrep -a -u "$(id -u)" 'patina|foot' | cut -c1-70
