#!/bin/sh
# Runs inside the VM. Starts the DRM session with the panel's Wayland protocol trace written to
# /tmp/panel.wl, to see what the panel is waiting for.
rm -f /tmp/panel.wl
export PANEL_CMD="WAYLAND_DEBUG=client WGPU_BACKEND=gl $HOME/PatineDE/target/release/patina-panel 2>/tmp/panel.wl"
exec sh "$HOME/PatineDE/ci/vm/guest/drm-up.sh"
