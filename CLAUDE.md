# PatineDE: project instructions

Rust Wayland desktop environment (Patina Desktop) for Debian, Ubuntu, Linux Mint, Fedora and Arch (x86_64, aarch64).

## Foundation
`design/Patina Desktop.dc.html` is the design foundation; every shell and app screen must match its layout, copy, behaviour, colours, type, spacing and radii. `design/tokens/*.css` is the source of truth for tokens (generated into `crates/patina-tokens`). Read `docs/PLAN.md` and `design/HANDOFF.md` first.

## Rules
- HTML files are references, not code to ship. Recreate them in Rust.
- Do not invent visuals. If the prototype does not cover something, propose a design in the same style and wait for approval.
- Keep the permission model: apps ask before mic, camera, screen capture, location or keyboard monitoring; X11 apps are isolated by default.
- `packaging/` is untested. Verify against real package names on each distro before relying on it.
- Bundled apps for other distros: Patina Terminal, File Bin, Settings. Cloudbreak Files and BeaconHub are IronTide OS only.
- Terminal runs a real PTY shell (the prototype only emulates one).
- Dev machine is macOS: Linux-only crates build in containers or QEMU. Keep platform-neutral crates (tokens, ui logic) buildable on macOS.
