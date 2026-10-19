# PatineDE

Patina Desktop: a Wayland desktop environment written in Rust. Installs as a normal session on Debian 12+, Ubuntu 22.04+, Linux Mint 21+/LMDE 6, Fedora and Arch (x86_64, aarch64).

Status: Milestone 0 (foundations). See [docs/PLAN.md](docs/PLAN.md).

## Layout
- `crates/patina-tokens`: design tokens generated from `design/tokens/*.css`
- `design/`: prototype, tokens and assets from the design handoff (reference only)
- `assets/fonts`: Plus Jakarta Sans and JetBrains Mono (SIL OFL, from Google Fonts)
- `packaging/`: Debian, Arch and shared session files (untested; Fedora spec to come)

## Install on Debian, Ubuntu and Linux Mint

Build the package on the release you want to install it on (a package built on a newer glibc
will not run on an older one), then install it:

```sh
packaging/debian/build.sh --bootstrap-rust --install-build-deps
sudo apt install ./dist/patina-desktop_0.1.0_*.deb
```

Then log out and choose **Patina** in the session menu of GDM or SDDM, or run
`patina-session` from a text console (Ctrl+Alt+F3, log in). LightDM, the default on Linux
Mint, cannot start Wayland sessions, so use the console on Mint.

The build needs network access: it installs the Rust toolchain pinned in
`packaging/RUST_VERSION` (the Rust in Debian 12 and Ubuntu 22.04 is too old) and downloads the
crates in `Cargo.lock`. Logs are in `~/.local/state/patina/session.log`.

This is a pre-release: compositor and top panel only, and the panel shows placeholder data.

## Develop
```sh
cargo test --workspace
```
