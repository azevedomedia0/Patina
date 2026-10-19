# CI

`.github/workflows/ci.yml` checks every push and pull request in each target distro's own
container: `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test`, for the whole
workspace. System packages come from `ci/deps.sh` (one list per package manager) and the Rust
release from `rust-toolchain.toml`, which must match `packaging/RUST_VERSION`.

| Distro | x86_64 | aarch64 |
| --- | --- | --- |
| Debian 12 | yes | yes |
| Ubuntu 22.04 | yes | |
| Ubuntu 24.04 | yes | |
| Linux Mint 21 (`linuxmintd/mint21.3-amd64`) | yes | |
| Fedora (latest) | yes | yes |
| Arch (latest) | yes | |

First run on GitHub, 2026-10-19: all eight jobs passed, about three minutes each.

Without GitHub, `ci/local-matrix.sh` runs the same steps in the same images with podman, inside
the build VM (`docs/VM.md`). Native arm64 entries take a couple of minutes; the x86_64-only
images (Mint, Arch) run through qemu-user and are far slower. Locally verified the same day:
Debian 12, Ubuntu 22.04 and Ubuntu 24.04 on arm64.

Packages are not built in CI yet; see `docs/PACKAGING.md` for how they are built and tested.
