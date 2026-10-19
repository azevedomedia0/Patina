#!/bin/sh
# Runs inside the VM. Starts sway (headless output, real virtio input), then
# patina-compositor nested in it with the panel and test clients (default: two weston-simple-shm).
export XDG_RUNTIME_DIR=/tmp/xdg-patina
# Process names are truncated to 15 characters, so match full command lines. The [x] bracket
# trick stops pkill matching its own shell.
for p in '^sway' '[t]arget/release/patina-compositor' '[t]arget/release/patina-panel' '^foot' '^weston-simple-shm' '^swaybg'; do pkill -u "$(id -u)" -f "$p" >/dev/null 2>&1; done
sleep 1
rm -rf "$XDG_RUNTIME_DIR"; mkdir -p "$XDG_RUNTIME_DIR"; chmod 700 "$XDG_RUNTIME_DIR"
export WLR_BACKENDS=headless,libinput LIBSEAT_BACKEND=seatd WLR_RENDERER=pixman
cat > /tmp/patina-sway.conf <<CONF
output HEADLESS-1 resolution 1280x720
output * bg #2f5fa8 solid_color
default_border none
CONF
setsid sway -c /tmp/patina-sway.conf >/tmp/patina-sway.log 2>&1 &
sleep 3
export WAYLAND_DISPLAY=$(ls "$XDG_RUNTIME_DIR" | grep '^wayland-[0-9]*$' | head -1)
export SWAYSOCK=$(ls "$XDG_RUNTIME_DIR"/sway-ipc.*.sock | head -1)
echo "sway display: $WAYLAND_DISPLAY"
BIN=$HOME/PatineDE/target/release
[ $# -eq 0 ] && set -- weston-simple-shm weston-simple-shm
setsid env RUST_LOG=info,patina_compositor=debug "$BIN/patina-compositor" "WGPU_BACKEND=gl $BIN/patina-panel" "$@" >/tmp/patina-comp.log 2>&1 &
sleep 6
echo "--- compositor log"; head -20 /tmp/patina-comp.log
pgrep -a -u "$(id -u)" 'sway|patina|weston' | cut -c1-80
