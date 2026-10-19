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
Known gaps in M1: client corners are not clipped to the 20 px radius, no window shadow, no blur behind panel or windows, no popups or menus, windows are not re-clamped when the panel appears late, no HiDPI, no animations, windows are placed by a simple cascade, and the DRM/KMS backend (Milestone 2) is not started.
Decision made without sign-off: the compositor draws every titlebar, including for our own apps (consistent look, and blur only works compositor-side). Revisit if you want apps to draw their own.
Open: pin Rust version, verify Mint image names, Fedora spec, GPU testing.

## Not designed yet (propose in-style, then wait for approval)
Input methods, RTL, workspace overview and Alt+Tab, calendar/weather popover, Wi-Fi/Bluetooth pairing, light-theme pass, File Bin file operations, wallpaper picker.
