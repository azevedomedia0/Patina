# Packaging

## Debian, Ubuntu, Linux Mint (`packaging/debian`)

```sh
packaging/debian/build.sh --bootstrap-rust --install-build-deps   # result in dist/
packaging/debian/build.sh --bootstrap-rust --install-build-deps --host-arch amd64   # cross-build
```

`build.sh` installs the Rust toolchain pinned in `packaging/RUST_VERSION`, installs the
build dependencies from `debian/control` with `mk-build-deps`, symlinks `packaging/debian` to
`./debian` for the duration of the build, and runs `dpkg-buildpackage -us -uc -b`.

What the package contains: `/usr/bin/patina-compositor`, `/usr/bin/patina-panel`,
`/usr/bin/patina-session`, `/usr/share/wayland-sessions/patina.desktop`, man pages, the
copyright file and a list of the Rust crates linked in. Fonts and icons are compiled into the
binaries, so no font packages are needed.

The pointer theme is installed as `/usr/share/icons/Patina` (an XCursor theme with the design's
nine cursors plus the usual X11 and CSS names for them; the session sets `XCURSOR_THEME=Patina`).
It inherits Adwaita for cursors it lacks (horizontal and vertical resize, crosshair, zoom). The
theme is generated from `design/assets/cursors/*.png` by `packaging/common/make-cursor-theme.py`
(needs Pillow) and the output in `assets/cursors/Patina` is committed, so building a package
needs no Python imaging library. Run the script again whenever a cursor image changes.

Decisions worth knowing:
- **Rust toolchain.** Debian 12 ships Rust 1.63 and Ubuntu 22.04 ships 1.75; the code needs
  1.85. The package therefore builds with a pinned rustup toolchain and needs network access
  to crates.io. That is fine for an out-of-archive package and would need vendoring to enter
  the Debian archive.
- **One build per release.** `dh_shlibdeps` currently computes glibc 2.35 as the minimum, but
  build on each target release anyway.
- **Cross-building.** `--host-arch` (or `dpkg-buildpackage -a`) builds for the other
  architecture with the matching `crossbuild-essential` package and multiarch libraries; the
  rules pick the Rust target and linker. Tests are skipped in a cross build.
- **Compression.** Packages use xz, which every supported dpkg reads (Debian 12's lintian
  rejects the zstd that Ubuntu's dpkg uses by default).
- **Services are Recommends, not Depends.** NetworkManager, BlueZ, PipeWire and the others are
  not used yet. `foot` is recommended so a fresh session has a terminal until Patina Terminal
  exists.
- **Licence.** The copyright file says the project code is GPL-3+, taken from the PKGBUILD in
  the design handoff. There is no `LICENSE` file yet; decide before releasing.
- **Maintainer and homepage** are placeholders (`patina@example.invalid`).

### What has been tested

Packages are built in pristine chroots (minimal base plus `Build-Depends` only) by
`ci/vm/guest/chroot-build.sh`, checked with `lintian --fail-on error,warning`, and installed into
a bare chroot of the same release by `ci/vm/guest/install-test.sh` (apt resolves the
dependencies, `ldd` finds every library, purge leaves nothing behind).

| Release | Arch | Build | lintian | Bare install and purge |
| --- | --- | --- | --- | --- |
| Debian 12 bookworm | arm64 | passes, tests run | clean | passes |
| Debian 13 trixie | arm64 | passes, tests run | clean | passes |
| Ubuntu 22.04 jammy (base of Mint 21) | arm64 | passes, tests run | clean | passes |
| Ubuntu 24.04 noble (base of Mint 22) | arm64 | passes, tests run | clean | passes |
| Debian 12 bookworm | amd64 (cross-built on arm64) | passes, tests skipped | clean | passes (under qemu-user) |

Running it:
- Installed package on the VM's display from a text console (`patina-session`): works.
- Login screens, each set to log user `dev` into the "Patina" session automatically
  (`ci/vm/guest/dm-test.sh`): SDDM, GDM and LightDM all start it as a logind session of type
  `wayland` on `seat0` through libseat's logind backend (no seatd), with a panel on each display.
  GDM leaves logind's `Desktop` field empty (SDDM and LightDM set it from `DesktopNames`); the
  session's own `XDG_CURRENT_DESKTOP` is set either way. Autologin skips the greeter, so
  picking the session by hand at each greeter was not tested.
- The amd64 build was started nested inside the arm64 session through qemu-user
  (`ci/vm/guest/amd64-smoke.sh`): compositor and panel start and draw. It has not run on real
  amd64 hardware.

Not tested: Ubuntu and Mint amd64 builds (Ubuntu keeps amd64 and arm64 on different mirrors,
so the cross-build chroot only does Debian), a real Linux Mint install, a real GPU, removal
and purge on a system with a running session.

### Signing in with greetd

The package includes Patina's own sign-in screen for greetd. To use it instead of GDM, SDDM or
LightDM: `sudo apt install greetd`, copy `/usr/share/doc/patina-desktop/greetd-config.toml` to
`/etc/greetd/config.toml`, disable the other display manager and enable `greetd`. Tested in the
Debian 12 VM.

### Known limits
- On the software-rendered VM the panel takes several seconds to appear.
- `packaging/arch/PKGBUILD` is still the untested handoff stub (it builds a `patina-launcher`
  crate that does not exist and uses version 0.4.0); it is the next packaging job, then Fedora.
