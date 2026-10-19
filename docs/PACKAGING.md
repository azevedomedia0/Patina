# Packaging

## Debian, Ubuntu, Linux Mint (`packaging/debian`)

```sh
packaging/debian/build.sh --bootstrap-rust --install-build-deps   # result in dist/
```

`build.sh` installs the Rust toolchain pinned in `packaging/RUST_VERSION`, installs the
build dependencies from `debian/control` with `mk-build-deps`, symlinks `packaging/debian` to
`./debian` for the duration of the build, and runs `dpkg-buildpackage -us -uc -b`.

What the package contains: `/usr/bin/patina-compositor`, `/usr/bin/patina-panel`,
`/usr/bin/patina-session`, `/usr/share/wayland-sessions/patina.desktop`, man pages, the
copyright file and a list of the Rust crates linked in. Fonts and icons are compiled into the
binaries, so no font packages are needed.

Decisions worth knowing:
- **Rust toolchain.** Debian 12 ships Rust 1.63 and Ubuntu 22.04 ships 1.75; the code needs
  1.85. The package therefore builds with a pinned rustup toolchain and needs network access
  to crates.io. That is fine for an out-of-archive package and would need vendoring to enter
  the Debian archive.
- **One build per release.** `dh_shlibdeps` currently computes glibc 2.35 as the minimum, but
  build on each target release anyway.
- **Services are Recommends, not Depends.** NetworkManager, BlueZ, PipeWire and the others are
  not used yet. `foot` is recommended so a fresh session has a terminal until Patina Terminal
  exists.
- **Licence.** The copyright file says the project code is GPL-3+, taken from the PKGBUILD in
  the design handoff. There is no `LICENSE` file yet; decide before releasing.
- **Maintainer and homepage** are placeholders (`patina@example.invalid`).

### What has been tested

| Check | Result |
| --- | --- |
| Build in a pristine Debian 12 arm64 chroot (minimal base, only `Build-Depends` installed) | passes, tests run during the build |
| `lintian --pedantic` | no output (two info-level tags overridden with reasons) |
| Install into a bare Debian 12 chroot | all dependencies resolve, no unresolved shared libraries |
| Install in the VM and start `/usr/bin/patina-session` on the display | works |

Not tested: amd64, Ubuntu 22.04 and 24.04, Linux Mint, GDM and SDDM session entries, a real
GPU, removal and purge on a system with a running session.

### Known limits
- LightDM (the Linux Mint default) cannot start Wayland sessions.
- On the software-rendered VM the panel takes several seconds to appear.
- `packaging/arch/PKGBUILD` is still the untested handoff stub (it builds a `patina-launcher`
  crate that does not exist and uses version 0.4.0); it is the next packaging job, then Fedora.
