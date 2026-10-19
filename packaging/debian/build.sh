#!/bin/sh
# Builds patina-desktop_<version>_<arch>.deb for the distro you run it on (Debian, Ubuntu,
# Linux Mint). The package is built per distro release: a build made on a newer glibc will
# not run on an older one.
#
# Usage: packaging/debian/build.sh [--bootstrap-rust] [--install-build-deps]
#   --bootstrap-rust       install rustup if it is missing (downloads from sh.rustup.rs)
#   --install-build-deps   apt-install the build dependencies from debian/control (needs sudo)
#
# The result is written to dist/. The build needs network access: it installs the Rust
# toolchain pinned in packaging/RUST_VERSION and downloads the crates in Cargo.lock.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"
RUST_VERSION=$(cat packaging/RUST_VERSION)
BOOTSTRAP=0
DEPS=0
for arg in "$@"; do
  case $arg in
    --bootstrap-rust) BOOTSTRAP=1 ;;
    --install-build-deps) DEPS=1 ;;
    -h | --help) sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done

# dpkg-buildpackage needs ./debian. The packaging lives in packaging/debian, so link it for
# the duration of the build, and refuse to touch a real directory.
if [ -e debian ] && [ ! -L debian ]; then
  echo "error: ./debian exists and is not a symlink; refusing to replace it" >&2
  exit 1
fi
ln -sfn packaging/debian debian
trap 'rm -f debian' EXIT

# Rust: the distro's cargo is too old (Debian 12: 1.63, Ubuntu 22.04: 1.75).
export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v rustup >/dev/null 2>&1; then
  if [ "$BOOTSTRAP" -eq 1 ]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
    export PATH="$HOME/.cargo/bin:$PATH"
  else
    echo "error: rustup not found. Install it from https://rustup.rs or pass --bootstrap-rust." >&2
    exit 1
  fi
fi
rustup toolchain install "$RUST_VERSION" --profile minimal
export RUSTUP_TOOLCHAIN="$RUST_VERSION"

if [ "$DEPS" -eq 1 ]; then
  if [ "$(id -u)" -eq 0 ]; then SUDO=""; else SUDO="sudo"; fi
  $SUDO apt-get install -y --no-install-recommends build-essential devscripts equivs
  work=$(mktemp -d)
  # mk-build-deps writes a small .deb into the current directory, so run it somewhere disposable.
  (cd "$work" && $SUDO mk-build-deps --install --remove \
    --tool 'apt-get -y --no-install-recommends' "$ROOT/debian/control")
  rm -rf "$work"
fi

dpkg-buildpackage -us -uc -b

mkdir -p dist
mv ../patina-desktop_* dist/
echo "built:"
ls -1 dist/patina-desktop_*.deb
