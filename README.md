# PatineDE

Patina Desktop: a Wayland desktop environment written in Rust. Installs as a normal session on Debian 12+, Ubuntu 22.04+, Linux Mint 21+/LMDE 6, Fedora and Arch (x86_64, aarch64).

Status: Milestone 0 (foundations). See [docs/PLAN.md](docs/PLAN.md).

## Layout
- `crates/patina-tokens`: design tokens generated from `design/tokens/*.css`
- `design/`: prototype, tokens and assets from the design handoff (reference only)
- `assets/fonts`: Plus Jakarta Sans and JetBrains Mono (SIL OFL, from Google Fonts)
- `packaging/`: Debian, Arch and shared session files (untested; Fedora spec to come)

## Develop
```sh
cargo test --workspace
```
