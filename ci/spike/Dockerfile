# Headless Wayland test bench for the toolkit spike. Debian 12 = oldest target.
FROM debian:12
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates git build-essential pkg-config \
    libwayland-dev libxkbcommon-dev libudev-dev libinput-dev libgbm-dev libseat-dev libegl1-mesa-dev \
    libvulkan1 mesa-vulkan-drivers libgl1-mesa-dri \
    sway grim imagemagick dbus \
 && rm -rf /var/lib/apt/lists/*
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
ENV PATH=/root/.cargo/bin:$PATH CARGO_TARGET_DIR=/cache/target
