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
Done: repo, fonts, token generator with tests, CI matrix (unrun), packaging stubs copied.
Open: toolkit spike (panel on layer-shell with blur), pin Rust version, verify Mint image names, Fedora spec, Linux build container for the compositor.

## Not designed yet (propose in-style, then wait for approval)
Input methods, RTL, workspace overview and Alt+Tab, calendar/weather popover, Wi-Fi/Bluetooth pairing, light-theme pass, File Bin file operations, wallpaper picker.
