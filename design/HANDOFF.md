# Handoff: Patina Desktop (new Claude Code project)

## Objective
Build **Patina Desktop**, a Rust-written Wayland desktop environment that installs on **Linux Mint, Ubuntu, Debian and Arch Linux** (x86_64 and aarch64). It is the shell of IronTide OS but must also install as a normal session on those four distros.

Start a new Claude Code project in an empty repo and give it this folder. First task for Claude Code: read this README, then propose the workspace layout and milestone plan before writing code.

## About the design files
`Patina Desktop.dc.html` and `Patina Install.dc.html` are **HTML design references**, prototypes of the intended look and behaviour. They are not production code. Recreate them in the real stack (below). Packaging files in `packaging/` are untested starting points.

## Fidelity
**High-fidelity** for visuals: colours, type, spacing and radii come from the Patina Desktop design system (`_ds/`). Behaviour is simulated with sample data.

## Recommended stack
- Rust workspace. Compositor on **Smithay** (DRM/KMS, libinput, seatd/logind, GBM/EGL, xdg-shell, layer-shell). XWayland for X11 apps.
- Shell UI (panel, dock, launcher, quick settings, lock/login): a Rust GUI toolkit that can render blur and 120-180 ms animations (for example iced or a custom wgpu layer). Decide in milestone 0.
- Services: D-Bus (zbus), PipeWire/WirePlumber, NetworkManager, BlueZ, UPower, polkit, XDG portals, a notification daemon, logind for sessions.
- Greeter: greetd with a Patina greeter, or a session entry for GDM, LightDM and SDDM.

## Target distros
- **Debian 12+**: .deb via `packaging/debian`. Rust 1.75+ needs backports or rustup on Bookworm.
- **Ubuntu 22.04+ / Linux Mint 21+ / LMDE 6**: same .deb. Check each release's Rust and library versions.
- **Arch**: `packaging/arch/PKGBUILD`.
- All: install `/usr/share/wayland-sessions/patina.desktop` and `/usr/bin/patina-session` (in `packaging/common/`). Binaries assumed: patina-compositor, patina-panel, patina-launcher. Rename to match the real crates.
- Later: Flatpak for apps, CI that builds and smoke-tests in Debian, Ubuntu, Mint and Arch containers and VMs.

## Milestones (suggested order)
0. Repo, toolchain, CI, cross-compile (x86_64, aarch64), decide GUI toolkit. The toolkit must support right-to-left mirroring, text shaping for Arabic and Hebrew, and a software-rendering fallback, because milestones 2, 7 and 8 depend on them.
1. Compositor: nested window (winit) first, then DRM/KMS; floating windows, focus, drag, resize, minimise, maximise. Also four workspaces (Super + 1-4) and the tiling toggle (Super + T).
2. Shell: top panel (36 px), floating dock (left or right, icon size, auto-hide), launcher grid, notifications with unread badge, quick settings. Panel clock options (12 h / 24 h, date) and the input-source indicator. Build layout direction-aware from the start so RTL mirroring is not a retrofit.
3. Session: login greeter, lock screen, log off, restart, shut down, switch user, autostart, idle and suspend. End of milestone: first installable .deb and PKGBUILD that start a Patina session, tested in Debian, Ubuntu, Mint and Arch VMs.
4. XWayland and permission prompts (screen capture, mic, camera, location, keyboard monitoring) via portals. Includes the amber in-use indicator, per-app Ask / Allow / Deny rules, the X11 isolation default, and the XWayland on/off and scaling options.
5. Core apps: File Bin (files), System (processes), Terminal.
6. Settings app and system services wiring. Pages: Display, Sound, Network, Appearance, Keyboard, Power, Language & region, Privacy, X11 apps, Dock & panel. Wire each to PipeWire, NetworkManager, BlueZ, UPower and logind.
7. Input and i18n: keyboard layouts (US, UK, DE, Arabic, Hebrew), input methods (IME, Japanese romaji to hiragana as the reference case), source switching (Ctrl + Space or the panel indicator), RTL layout and text (Auto / LTR / RTL), interface localisation (English, Deutsch, العربية, עברית).
8. Accessibility (text size, reduce motion), light theme pass (not yet designed), low-spec mode (software rendering), packaging for all four distros, QA.
Full checklist: the 19 engineering items in the project todo list (compositor, window management, rendering, XWayland, shell UI, quick settings, toolkit, settings app, system services, session, core apps, packaging, input and i18n, accessibility, security, performance, testing, docs).

## Screens and behaviour in the prototype (`Patina Desktop.dc.html`)
- **Shell**: top panel with launcher button, workspace buttons, active app title, clock, status icons; vertical floating dock on the left with running indicators; launcher grid; notifications panel; quick settings popover (320 px).
- **Quick settings**: tiles for Wi-Fi, Bluetooth, VPN and Cast, each with a main toggle and a chevron that opens a device list below. Also volume, brightness, opacity. Footer icon buttons: Settings, Lock, Log off, Restart, Power.
- **Windows**: free-floating, draggable by titlebar, resizable from the bottom-right corner, minimise, maximise, close, double-click titlebar to maximise. Translucent ink backing with blur.
- **System app**: Processes tab plus settings pages (including Privacy and X11 apps), custom accent, shell and wallpaper colours with a rainbow label, brightness, dock and panel options, users.
- **File Bin**: sidebar with Places (Home, Documents, Downloads, Pictures, Music, Videos, Trash Bin), Devices (with eject) and Network; list area; always-visible resizable right panel with preview and metadata.
- **Terminal**: tabs, zoom, four colour profiles, right-click menu, emulated shell commands, history, tab completion. Commands that need mic, screen, camera or location raise permission prompts.
- **XEyes (X11 sample)**: legacy-app example for XWayland, with prompts for screen capture and keyboard monitoring.
- **Permissions**: modal prompt with Don't allow, Allow once, Always allow; amber panel indicator while in use; per-app rules in Settings, Privacy.
- **Session screens**: sign-in screen (large clock at top, avatar, password field, arrow button, switch-user bottom-left, shut down bottom-right), identical lock screen, restart and shutdown screens, boot screen with progress bar.
- **Right-click menus** on desktop, windows, files, terminal and dock.

## Design tokens
Use `_ds/patina-desktop-ad7af6dc-1540-4ce8-903e-584d99abf18a/tokens/*.css` as the source of truth (colors, typography, spacing, base, theme-light).
- Ink tile #1a1f29; blue ramp #2f7de1 to #c8dcfa; one accent (tide-500); semantic green, amber, red.
- Fonts: Plus Jakarta Sans (UI, 13 px body), JetBrains Mono (paths, clock, telemetry). Scale 10/11/12/13/15/20/32.
- Spacing 2, 4, 6, 8, 12, 16, 24, 32, 48. Panel 36 px, titlebar 38 px. Radii: pill for buttons and inputs, 10-14 for rows and cards, 20 for windows, 22% for app icons.
- Glass: translucent ink with 16-24 px blur. Motion 120-180 ms, cubic-bezier(.2,.8,.2,1).
- Icons: Lucide, stroke 2. App icons: squircle PNGs in `assets/`.

## Assets
`assets/files.png` (File Bin, user supplied), `assets/monitor.png` (System, user supplied), `assets/terminal.png` and `assets/xeyes.png` (drawn placeholders), `assets/cursors/` (nine cursors, user supplied, see Cursors below). No Patina or BeaconHub logos exist yet.

## Cursors
Nine user-supplied cursor images in `assets/cursors/` (48 x 48 RGBA PNG, light grey with soft shadow). Hotspots are in image pixels. In the Rust repo they are packaged as the XCursor theme `Patina` (`assets/cursors/Patina`, installed to `/usr/share/icons/Patina`, built by `packaging/common/make-cursor-theme.py`) using the CSS/XCursor names below, so the compositor's cursor handling and XWayland apps pick them up.

| File | Source name | Use (CSS / XCursor name) | Hotspot |
| --- | --- | --- | --- |
| `default.png` | Mouse Cursor | default (`default`, `left_ptr`) | 5, 3 |
| `pointer.png` | Link Select | links and buttons (`pointer`, `hand2`) | 10, 4 |
| `text.png` | Text Select | text fields (`text`, `xterm`) | 13, 24 |
| `move.png` | Move | window drag, titlebar (`move`, `grab`, `fleur`) | 24, 24 |
| `nwse-resize.png` | Diagonal Resize 1 | window corner resize, top-left / bottom-right (`nwse-resize`) | 24, 24 |
| `nesw-resize.png` | Diagonal Resize 2 | window corner resize, top-right / bottom-left (`nesw-resize`) | 24, 24 |
| `wait.png` | Mouse Cursor Busy | app starting or busy (`wait`, `progress`) | 19, 16 |
| `help.png` | Mouse Cursor Help | help (`help`) | 5, 3 |
| `not-allowed.png` | Mouse Cursor UNavailable | action unavailable (`not-allowed`) | 5, 3 |

In `Patina Desktop.dc.html` the default, pointer, move (drag handles), text and nwse-resize cursors are wired to the existing elements. The busy cursor (`wait`) shows across the whole lock screen. The sign-in screen keeps the normal arrow. `nesw-resize`, `help` and `not-allowed` are defined as classes (`.cur-nesw-resize`, `.cur-help`, `.cur-not-allowed`) but no prototype element uses them yet. The File Bin panel splitter keeps the browser's `ew-resize` because no horizontal or vertical resize cursor was supplied. Those are still needed (`ew-resize`, `ns-resize`), along with `grabbing`, and are not designed yet. `uploads/cursor.png` is the earlier arrow and is no longer referenced.

## Not designed yet
Workspace overview and Alt+Tab; BeaconHub app; calendar and weather popover; real Wi-Fi and Bluetooth pairing; light-theme pass; file operations in File Bin; wallpaper picker.

## Files
- `Patina Desktop.dc.html`: main prototype (open in a browser).
- `Patina Install.dc.html`: install instructions page.
- `packaging/`: debian, arch, common session files.
- `_ds/`: design system tokens, components, bundle.
- `assets/`, `uploads/`: images.
- `github.md`: source repo reference (azevedomedia0/Patina).
