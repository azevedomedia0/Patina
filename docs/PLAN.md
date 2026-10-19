# Plan

## Decisions (approved)
- Targets: Debian 12+, Ubuntu 22.04+, Linux Mint 21+/LMDE 6, Fedora, Arch; x86_64 and aarch64 (aarch64 tested under QEMU).
- Bundled apps on other distros: Patina Terminal, File Bin, Settings. Cloudbreak Files and BeaconHub are IronTide OS only.
- Terminal: real PTY shell.
- Toolkit: iced + iced_layershell for shell surfaces and apps, custom wgpu pass for blur. Milestone 0 spike confirms; fallback is custom wgpu.
- First login path: GDM/LightDM/SDDM session entry first, greetd greeter later (user undecided; this was the recommendation).
- Fonts: Plus Jakarta Sans and JetBrains Mono, bundled from Google Fonts (OFL).

## Architecture
Smithay compositor plus separate layer-shell client processes (panel, dock, launcher, quick settings, session UI) so a crashed panel does not end the session. Services over zbus.

## Build constraints
Distro cargo is too old (Debian 12: 1.63, Ubuntu 22.04: 1.75). Packages build with a pinned rustup toolchain; release binaries are built in a Debian 12 container for the oldest glibc.

## Milestones
0. Repo, CI matrix, toolkit spike, token generator
1. Nested compositor (winit): window management
2. DRM/KMS, libinput, seatd/logind
3. Shell: panel, dock, launcher, notifications, quick settings
4. Session: session entry, lock, power, greetd greeter, idle/suspend
5. XWayland isolation, permission broker and prompts
6. Apps: File Bin, Settings (incl. Processes), Terminal
7. Settings pages, real Wi-Fi/Bluetooth/audio
8. Input/i18n, accessibility, light theme, low-spec, .deb/.rpm/PKGBUILD, QA

## Milestone 0 status
Done: repo, fonts, Lucide icons, token generator with tests, CI matrix (unrun), packaging stubs, toolkit spike (docs/SPIKE.md), QEMU build VM (docs/VM.md).
Milestone 1 (nested compositor), working in the QEMU VM (`docs/spike/window-frames.png`):
- xdg-shell windows with compositor-drawn 38 px titlebars (xdg-decoration forces server-side), layer-shell, viewporter.
- Move by titlebar, resize by the bottom-right grip, maximise (control or double-click titlebar), minimise (Alt+M restores until the dock exists), close, click-to-raise and focus, hover on controls, inactive windows dimmed to 92%.
- Titlebar drawing and hit-testing live in `patina-frame` (no Wayland dependency, tested on macOS).
Also: window content is clipped to the 20 px bottom corners by a GLES shader (`clip.frag`), and each window has the design's shadow and 1 px outline (`patina-frame::shadow`, `docs/spike/window-shadow-corners.png`).
Milestone 2 (DRM/KMS) started and working in the VM (`docs/spike/drm-session.png`): libseat session via seatd, libinput keyboard and pointer (absolute and relative), udev GPU selection, GBM/EGL renderer, `DrmCompositor` on the first connected output, a software cursor from the design's `cursor.png`, vblank-driven rendering, VT switching on Ctrl+Alt+F1..F12 (not yet exercised). The backend is chosen automatically: winit when run inside another session, DRM from a TTY (`PATINA_BACKEND=drm|winit` overrides). Run it with `ci/vm/guest/drm-up.sh`.
Debian packaging (`packaging/debian`, `docs/PACKAGING.md`): `patina-desktop_0.1.0_arm64.deb` builds in a pristine Debian 12 chroot, lintian is clean, installs from a bare system with all dependencies resolving, and the installed `/usr/bin/patina-session` starts a working session in the VM.
Rendering is event-driven: a frame is drawn only after a surface commit, input, window or layer change, or a title change; the idle compositor measures 0 wakeups/s and 0 ms CPU/s (it was 47/s and 13 ms/s with the old 16 ms retry timer; `ci/vm/guest/idle-wakeups.sh`). On drivers without fence support (virtio-gpu, software GL) the DRM backend waits on the render sync point before queueing a frame, as Smithay requires; skipping that showed half-drawn frames (a missing panel), which the old polling had hidden.
Cursors: the design's arrow is the default pointer; other named cursors come from the system XCursor theme (as the prototype uses stock cursors for everything but the arrow), client cursor surfaces are drawn as given, `cursor-shape` is supported, and the compositor shows the resize cursor over a window's corner grip. In the nested (winit) backend the host draws the pointer, so none of this applies there.
Panel: the clock (on the minute), battery (hidden when there is none), focused-window title (via `wlr-foreign-toplevel-management`, also needed by the dock) and the four workspace buttons (`ext-workspace-v1`; the compositor stashes windows per workspace) are live. Still not wired: the launcher button (no launcher yet), the wifi/bluetooth/brightness/volume icons (they show no state and open nothing: services and quick settings are milestone 3 and 7) and the notification bell (no notification service, so it is hidden). Super+1..4 switch workspace and Super+Shift+1..4 move the focused window there and follow it (physical number-row keys, so they work on AZERTY and similar). These shortcuts are my proposal, since the prototype has no keyboard handling for workspaces; they are non-visual and easy to change.
Glass: the desktop shows the design's `--wallpaper` (two radial glows, rendered from the CSS token in `patina-frame::wallpaper`), and the panel (`blur(16px)`) and every window (`blur(24px) saturate(1.4)`) blur what is behind them, drawn by the compositor. The frame is composed into an offscreen scene texture bottom to top (`scene.rs`); before each glass surface, the scene so far is downsampled to a quarter, blurred with a separable Gaussian and drawn back inside the surface's rounded rectangle (`blur.rs`, `gaussian.frag`, `backdrop.frag`). The scene is recomposed only when something in it changed (an element's id, commit counter, geometry or alpha), so an idle desktop stays idle (0 wakeups/s with blur on). `PATINA_NO_BLUR=1` turns it off. Verified A/B on the VM: bright content behind a titlebar is a soft haze with blur and a hard-edged block without it. Cost on the VM's software GL: a recompose takes 7-55 ms (median 33 ms) with three glass surfaces, so dragging runs at roughly 30 fps there; a real GPU should be far faster, but this is untested. Not done: partial recompose (any change recomposes everything), HiDPI-aware blur sizes beyond scaling sigma, blur for popups and menus (none exist yet).
Known gaps in M2: single GPU and single output, no hotplug, no multi-monitor,  no suspend/resume testing, software GL only in the VM (no real GPU tested), 
Known gaps in M1: no popups or menus, windows are not re-clamped when the panel appears late, no HiDPI, no animations, windows are placed by a simple cascade, and the DRM/KMS backend (Milestone 2) is not started.
Decision made without sign-off: the compositor draws every titlebar, including for our own apps (consistent look, and blur only works compositor-side). Revisit if you want apps to draw their own.
Open: pin Rust version, verify Mint image names, Fedora spec, GPU testing.

## Not designed yet (propose in-style, then wait for approval)
Input methods, RTL, workspace overview and Alt+Tab, calendar/weather popover, Wi-Fi/Bluetooth pairing, light-theme pass, File Bin file operations, wallpaper picker.
