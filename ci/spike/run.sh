#!/bin/sh
# Runs the panel on headless sway and screenshots it. Usage: run.sh [extra env, e.g. ICED_BACKEND=tiny-skia]
set -u
if [ "$(id -u)" = 0 ]; then
  id dev >/dev/null 2>&1 || useradd -m dev
  chmod 777 /out
  exec su dev -s /bin/sh -c "CLICK_X='${CLICK_X:-}' $0 $*"
fi
export XDG_RUNTIME_DIR=/tmp/xdg; mkdir -p "$XDG_RUNTIME_DIR"; chmod 700 "$XDG_RUNTIME_DIR"
export WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman
cat > /tmp/sway.conf <<CONF
output HEADLESS-1 resolution 1280x720
output * bg #2f5fa8 solid_color
CONF
sway -c /tmp/sway.conf >/tmp/sway.log 2>&1 &
sleep 3
export WAYLAND_DISPLAY=$(ls "$XDG_RUNTIME_DIR" | grep '^wayland-[0-9]*$' | head -1)
export SWAYSOCK=$(ls "$XDG_RUNTIME_DIR"/sway-ipc.*.sock | head -1)
echo "display: $WAYLAND_DISPLAY"
env "$@" /cache/target/release/patina-panel >/tmp/panel.log 2>&1 &
PANEL=$!
sleep 5
kill -0 $PANEL 2>/dev/null && echo "panel running" || echo "panel EXITED"
grim /out/idle.png && echo "screenshot ok"
swaymsg -t get_tree >/dev/null 2>&1; 
if [ -n "${CLICK_X:-}" ]; then
  swaymsg "seat seat0 cursor set $CLICK_X 18" >/dev/null
  swaymsg "seat seat0 cursor press button1" >/dev/null
  swaymsg "seat seat0 cursor release button1" >/dev/null
  sleep 0.05; grim /out/during.png
  sleep 1;    grim /out/after.png
fi
echo "--- panel log"; head -30 /tmp/panel.log
echo "--- sway log (tail)"; tail -5 /tmp/sway.log
