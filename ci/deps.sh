#!/bin/sh
# Installs what the workspace needs to build and test, for one package manager. Used by the CI
# workflow and by ci/local-matrix.sh, so the two cannot drift apart. Run as root.
#
# Usage: ci/deps.sh apt|dnf|pacman
set -eu
case ${1:?apt, dnf or pacman} in
  apt)
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      ca-certificates curl git build-essential pkg-config \
      libdrm-dev libgbm-dev libinput-dev libpam0g-dev libseat-dev libsystemd-dev libudev-dev \
      libwayland-dev libxkbcommon-dev
    ;;
  dnf)
    dnf install -y curl git gcc gcc-c++ make pkgconf-pkg-config \
      libdrm-devel mesa-libgbm-devel libinput-devel libseat-devel pam-devel systemd-devel \
      wayland-devel libxkbcommon-devel
    ;;
  pacman)
    pacman -Syu --noconfirm --needed curl git base-devel \
      libdrm mesa libinput pam seatd systemd-libs wayland libxkbcommon
    ;;
  *) echo "unknown package manager: $1" >&2; exit 2 ;;
esac
