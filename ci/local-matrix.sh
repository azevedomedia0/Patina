#!/bin/sh
# Runs the CI workflow's checks (fmt, clippy with warnings as errors, tests) in the same distro
# containers, without GitHub. Runs inside the build VM (docs/VM.md) with podman. Each entry
# starts from a fresh container, installs ci/deps.sh's packages and the toolchain pinned in
# rust-toolchain.toml, and builds a copy of the working tree.
#
# Usage: ci/local-matrix.sh [ENTRY...]   default: every entry below
#   ENTRY is NAME:IMAGE:PKG:PLATFORM. linux/amd64 entries on an arm64 VM run through qemu-user:
#   correct, but several times slower.
# Logs go to /tmp/matrix-<name>.log; a summary is printed at the end.
set -u
SRC=$(cd "$(dirname "$0")/.." && pwd)
DEFAULT="
debian12:docker.io/library/debian:12:apt:linux/arm64
ubuntu2204:docker.io/library/ubuntu:22.04:apt:linux/arm64
ubuntu2404:docker.io/library/ubuntu:24.04:apt:linux/arm64
fedora:docker.io/library/fedora:latest:dnf:linux/arm64
mint21:docker.io/linuxmintd/mint21.3-amd64:latest:apt:linux/amd64
arch:docker.io/library/archlinux:latest:pacman:linux/amd64
"
[ $# -gt 0 ] && DEFAULT="$*"

summary=""
for entry in $DEFAULT; do
  # NAME:REGISTRY/REPO:TAG:PKG:PLATFORM, where the image itself contains one colon.
  name=${entry%%:*}; rest=${entry#*:}
  platform=${rest##*:}; rest=${rest%:*}
  pkg=${rest##*:}; image=${rest%:*}
  log=/tmp/matrix-$name.log
  echo "=== $name ($image, $platform): $(date +%T)"
  start=$(date +%s)
  if podman run --rm --platform "$platform" -v "$SRC:/src:ro" "$image" sh -c '
      set -e
      sh /src/ci/deps.sh '"$pkg"' >/tmp/deps.log 2>&1 || { tail -30 /tmp/deps.log; exit 1; }
      mkdir /work && cd /src && tar cf - --exclude=./target --exclude=./dist --exclude=./.git \
        --exclude=./ci/vm/state . | tar xf - -C /work
      cd /work
      curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none >/dev/null
      . "$HOME/.cargo/env"
      rustup show active-toolchain || rustup toolchain install
      cargo fmt --all --check
      cargo clippy --locked --workspace --all-targets -- -D warnings
      cargo test --locked --workspace
    ' >"$log" 2>&1; then
    result="passed"
  else
    result="FAILED (see $log)"
  fi
  mins=$(( ($(date +%s) - start) / 60 ))
  echo "$name: $result in ${mins} min"
  summary="$summary
$name ($platform): $result, ${mins} min"
done
echo "=== summary$summary"
