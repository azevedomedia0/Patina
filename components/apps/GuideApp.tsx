'use client';

import React, { useState } from 'react';
import { BookOpen, Copy, Check, Terminal, Cpu, ShieldCheck } from 'lucide-react';

export default function GuideApp() {
  const [activeTab, setActiveTab] = useState<'rpi-arm32' | 'rpi-arm64' | 'arch-alpine' | 'direct-tty'>('rpi-arm32');
  const [copiedSnippet, setCopiedSnippet] = useState<string | null>(null);

  const copyCode = (code: string, id: string) => {
    navigator.clipboard.writeText(code);
    setCopiedSnippet(id);
    setTimeout(() => setCopiedSnippet(null), 2000);
  };

  return (
    <div className="flex h-full min-h-0 flex-col space-y-4 overflow-y-auto bg-neutral-950 p-4 font-sans text-xs text-neutral-200 select-none">
      {/* Title */}
      <div className="p-3 bg-neutral-900 border border-neutral-800 rounded-lg">
        <div className="flex items-center gap-2">
          <BookOpen className="w-4 h-4 text-amber-400" />
          <span className="font-semibold text-white text-sm">OxideDE Deployment & Hardware Guide</span>
        </div>
        <p className="text-[11px] text-neutral-400 mt-1">
          Complete instructions to build and launch OxideDE on 32-bit & 64-bit ARM single-board computers and low-end x86 machines.
        </p>
      </div>

      {/* Tabs */}
      <div className="flex items-center gap-1 border-b border-neutral-800 pb-2 overflow-x-auto">
        <button
          onClick={() => setActiveTab('rpi-arm32')}
          className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
            activeTab === 'rpi-arm32' ? 'bg-amber-500/20 text-amber-400 border border-amber-500/30' : 'text-neutral-400 hover:text-white'
          }`}
        >
          Raspberry Pi 2/3 (32-bit ARMv7)
        </button>
        <button
          onClick={() => setActiveTab('rpi-arm64')}
          className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
            activeTab === 'rpi-arm64' ? 'bg-amber-500/20 text-amber-400 border border-amber-500/30' : 'text-neutral-400 hover:text-white'
          }`}
        >
          Raspberry Pi 4/5 (64-bit ARM64)
        </button>
        <button
          onClick={() => setActiveTab('arch-alpine')}
          className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
            activeTab === 'arch-alpine' ? 'bg-amber-500/20 text-amber-400 border border-amber-500/30' : 'text-neutral-400 hover:text-white'
          }`}
        >
          Alpine / Arch Linux
        </button>
        <button
          onClick={() => setActiveTab('direct-tty')}
          className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
            activeTab === 'direct-tty' ? 'bg-amber-500/20 text-amber-400 border border-amber-500/30' : 'text-neutral-400 hover:text-white'
          }`}
        >
          Direct TTY Launch (seatd)
        </button>
      </div>

      {/* Tab Content */}
      <div className="space-y-4">
        {activeTab === 'rpi-arm32' && (
          <div className="space-y-3">
            <div className="p-3 bg-neutral-900 border border-neutral-800 rounded-lg space-y-2">
              <div className="font-semibold text-white">Target: Raspberry Pi 2, Pi 3 (armhf / armv7l)</div>
              <p className="text-neutral-400 leading-relaxed">
                Older 32-bit ARM devices often have only 512MB to 1GB of total RAM. GNOME or KDE are virtually unrunnable. OxideDE operates using a direct-scanout software rasterizer (<code className="text-amber-300">tiny-skia</code> + NEON SIMD) requiring only ~28MB of RAM.
              </p>
            </div>

            <div className="space-y-1">
              <div className="flex items-center justify-between text-neutral-300 font-semibold">
                <span>1. Install Native Build Dependencies (or Cross-Compile)</span>
                <button
                  onClick={() => copyCode('sudo apt update && sudo apt install -y build-essential curl git libwayland-dev libxkbcommon-dev libseat-dev libinput-dev libpixman-1-dev seatd', 'c1')}
                  className="text-neutral-400 hover:text-white flex items-center gap-1"
                >
                  {copiedSnippet === 'c1' ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                  <span>Copy</span>
                </button>
              </div>
              <pre className="p-2.5 bg-neutral-900 rounded border border-neutral-800 font-mono text-[11px] text-neutral-200 overflow-x-auto">
{`sudo apt update && sudo apt install -y \\
    build-essential curl git libwayland-dev \\
    libxkbcommon-dev libseat-dev libinput-dev \\
    libpixman-1-dev seatd`}
              </pre>
            </div>

            <div className="space-y-1">
              <div className="flex items-center justify-between text-neutral-300 font-semibold">
                <span>2. Build with NEON SIMD & Cache Optimizations</span>
                <button
                  onClick={() => copyCode('RUSTFLAGS="-C target-cpu=cortex-a7 -C target-feature=+neon" cargo build --profile armv7', 'c2')}
                  className="text-neutral-400 hover:text-white flex items-center gap-1"
                >
                  {copiedSnippet === 'c2' ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                  <span>Copy</span>
                </button>
              </div>
              <pre className="p-2.5 bg-neutral-900 rounded border border-neutral-800 font-mono text-[11px] text-neutral-200 overflow-x-auto">
{`# Uses profile.armv7 with opt-level = "z" and full LTO
RUSTFLAGS="-C target-cpu=cortex-a7 -C target-feature=+neon" \\
cargo build --profile armv7`}
              </pre>
            </div>
          </div>
        )}

        {activeTab === 'rpi-arm64' && (
          <div className="space-y-3">
            <div className="p-3 bg-neutral-900 border border-neutral-800 rounded-lg space-y-2">
              <div className="font-semibold text-white">Target: Raspberry Pi 4, Pi 5, Rockchip RK3588 (aarch64)</div>
              <p className="text-neutral-400 leading-relaxed">
                64-bit ARM boards feature OpenGL ES 3.1 hardware acceleration via the Mesa <code className="text-amber-300">v3d</code> or <code className="text-amber-300">panfrost</code> driver. OxideDE leverages hardware shaders and fractional scaling with sub-1ms render passes.
              </p>
            </div>

            <div className="space-y-1">
              <div className="flex items-center justify-between text-neutral-300 font-semibold">
                <span>Compile for aarch64 with full LTO</span>
              </div>
              <pre className="p-2.5 bg-neutral-900 rounded border border-neutral-800 font-mono text-[11px] text-neutral-200 overflow-x-auto">
{`cargo build --release
sudo cp target/release/oxide-compositor /usr/local/bin/
sudo cp target/release/oxide-panel /usr/local/bin/
sudo cp target/release/oxide-launcher /usr/local/bin/`}
              </pre>
            </div>
          </div>
        )}

        {activeTab === 'arch-alpine' && (
          <div className="space-y-3">
            <div className="p-3 bg-neutral-900 border border-neutral-800 rounded-lg space-y-2">
              <div className="font-semibold text-white">Alpine Linux (musl) & Arch Linux ARM</div>
              <p className="text-neutral-400 leading-relaxed">
                OxideDE statically compiles against musl libc on Alpine for an ultra-secure, container-ready or embedded kiosk distribution.
              </p>
            </div>

            <pre className="p-2.5 bg-neutral-900 rounded border border-neutral-800 font-mono text-[11px] text-neutral-200 overflow-x-auto">
{`# Alpine Linux APK Installation
apk add seatd wayland-dev libxkbcommon-dev libinput-dev pixman-dev
rc-service seatd start
rc-update add seatd default`}
            </pre>
          </div>
        )}

        {activeTab === 'direct-tty' && (
          <div className="space-y-3">
            <div className="p-3 bg-neutral-900 border border-neutral-800 rounded-lg space-y-2">
              <div className="font-semibold text-white">Direct Launch without Heavy Display Managers</div>
              <p className="text-neutral-400 leading-relaxed">
                To maximize free memory on 1GB devices, avoid heavy login managers like GDM or SDDM. Run OxideDE directly using <code className="text-amber-300">seatd-launch</code>.
              </p>
            </div>

            <pre className="p-2.5 bg-neutral-900 rounded border border-neutral-800 font-mono text-[11px] text-neutral-200 overflow-x-auto">
{`# Add user to the video and seat groups
sudo usermod -aG video,input,seat $USER

# Launch OxideDE directly from TTY1
seatd-launch oxide-compositor`}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
