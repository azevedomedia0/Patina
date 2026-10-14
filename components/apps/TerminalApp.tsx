'use client';

import React, { useState, useRef, useEffect } from 'react';
import { HardwareProfile, DEConfig } from '@/lib/oxide-types';
import { Terminal as TerminalIcon, Sparkles, CornerDownLeft } from 'lucide-react';

interface TerminalAppProps {
  hardware: HardwareProfile;
  config: DEConfig;
  onUpdateConfig?: (partial: Partial<DEConfig>) => void;
}

interface CommandHistoryItem {
  id: string;
  command: string;
  output: React.ReactNode;
}

export default function TerminalApp({ hardware, config, onUpdateConfig }: TerminalAppProps) {
  const [inputVal, setInputVal] = useState('');
  const [history, setHistory] = useState<CommandHistoryItem[]>([
    {
      id: 'init-1',
      command: 'fastfetch',
      output: (
        <div className="font-mono text-xs leading-relaxed space-y-1">
          <div className="text-amber-400 font-semibold">OxideDE v0.9.4 (Smithay Wayland Compositor)</div>
          <div className="text-neutral-400">-------------------------------------------</div>
          <div><span className="text-neutral-400">OS:</span> Oxide Linux (Alpine/Debian base)</div>
          <div><span className="text-neutral-400">Host:</span> {hardware.name} ({hardware.tagline})</div>
          <div><span className="text-neutral-400">Kernel:</span> 6.6.28-oxide-arm</div>
          <div><span className="text-neutral-400">Arch:</span> <span className="text-emerald-400 font-mono">{hardware.architecture}</span> ({hardware.bits})</div>
          <div><span className="text-neutral-400">Compositor:</span> oxide-compositor (Rust Smithay 0.3)</div>
          <div><span className="text-neutral-400">Rendering Backend:</span> <span className="text-cyan-400">{hardware.compositorBackend}</span></div>
          <div><span className="text-neutral-400">RAM:</span> <span className="text-emerald-400 font-semibold tabular-nums">{config.lowSpecMode ? hardware.idleRamMB - 6 : hardware.idleRamMB} MiB</span> / {hardware.ramTotalMB} MiB ({(((config.lowSpecMode ? hardware.idleRamMB - 6 : hardware.idleRamMB) / hardware.ramTotalMB) * 100).toFixed(1)}%)</div>
          <div><span className="text-neutral-400">Damage Tracking:</span> {config.damageTracking ? 'Active (DamageTracker)' : 'Disabled'}</div>
          <div><span className="text-neutral-400">Low-Spec Mode:</span> {config.lowSpecMode ? 'Enabled (Direct scanout, software blit)' : 'Standard'}</div>
          <div className="pt-1 text-neutral-400">Type <span className="text-amber-300 font-semibold">help</span> to view available commands.</div>
        </div>
      ),
    },
  ]);

  const terminalEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    terminalEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [history]);

  const handleCommand = (e: React.FormEvent) => {
    e.preventDefault();
    const cmd = inputVal.trim();
    if (!cmd) return;

    const parts = cmd.split(' ');
    const root = parts[0].toLowerCase();
    const args = parts.slice(1);

    let output: React.ReactNode = null;

    switch (root) {
      case 'help':
        output = (
          <div className="text-xs space-y-1 text-neutral-300 font-mono">
            <div className="text-amber-400 font-semibold">OxideDE Terminal - Built-in Commands:</div>
            <div><span className="text-cyan-300 font-bold">fastfetch / neofetch</span> - Display system, kernel, and compositor specs</div>
            <div><span className="text-cyan-300 font-bold">free -m</span> - Inspect RAM and swap allocations</div>
            <div><span className="text-cyan-300 font-bold">uname -a</span> - Print kernel and machine architecture</div>
            <div><span className="text-cyan-300 font-bold">cargo build --release</span> - Simulate compiling OxideDE for {hardware.architecture}</div>
            <div><span className="text-cyan-300 font-bold">oxide-ctl --status</span> - Query compositor frame times and renderer pipeline</div>
            <div><span className="text-cyan-300 font-bold">oxide-ctl --low-spec [on|off]</span> - Toggle ultra-low spec ARM optimization</div>
            <div><span className="text-cyan-300 font-bold">rustc --version</span> - Check compiler toolchain</div>
            <div><span className="text-cyan-300 font-bold">ls / cat [file]</span> - View local configuration</div>
            <div><span className="text-cyan-300 font-bold">clear</span> - Clear terminal screen</div>
          </div>
        );
        break;

      case 'clear':
        setHistory([]);
        setInputVal('');
        return;

      case 'fastfetch':
      case 'neofetch':
        output = (
          <div className="font-mono text-xs leading-relaxed space-y-1">
            <div className="text-amber-400 font-semibold">OxideDE v0.9.4 on {hardware.architecture}</div>
            <div><span className="text-neutral-400">Host:</span> {hardware.name}</div>
            <div><span className="text-neutral-400">CPU:</span> {hardware.cpuModel} ({hardware.cpuCores} cores)</div>
            <div><span className="text-neutral-400">GPU/Driver:</span> {hardware.gpuDriver}</div>
            <div><span className="text-neutral-400">Compositor:</span> oxide-compositor (Smithay Wayland)</div>
            <div><span className="text-neutral-400">Pipeline:</span> {hardware.compositorBackend}</div>
            <div><span className="text-neutral-400">RAM (idle):</span> {config.lowSpecMode ? hardware.idleRamMB - 6 : hardware.idleRamMB} MiB / {hardware.ramTotalMB} MiB</div>
          </div>
        );
        break;

      case 'uname':
        if (args.includes('-a') || args.length === 0) {
          output = (
            <div className="text-xs font-mono text-neutral-300">
              Linux oxide-box 6.6.28-oxide-arm #1 SMP PREEMPT_DYNAMIC {hardware.architecture} GNU/Linux
            </div>
          );
        } else if (args.includes('-m')) {
          output = <div className="text-xs font-mono text-emerald-400">{hardware.architecture}</div>;
        } else {
          output = <div className="text-xs font-mono text-neutral-300">Linux</div>;
        }
        break;

      case 'free':
        const usedRam = config.lowSpecMode ? hardware.idleRamMB - 6 : hardware.idleRamMB;
        const freeRam = hardware.ramTotalMB - usedRam - 64;
        output = (
          <div className="text-xs font-mono text-neutral-200 tabular-nums">
            <pre className="overflow-x-auto">
{`               total        used        free      shared  buff/cache   available
Mem:            ${hardware.ramTotalMB}          ${usedRam}         ${freeRam}           2          64        ${freeRam + 40}
Swap:            512           0         512`}
            </pre>
            <div className="text-[11px] text-emerald-400 pt-1">
              ✓ OxideDE footprint is {usedRam} MiB — over 12x leaner than GNOME (~480 MiB)!
            </div>
          </div>
        );
        break;

      case 'rustc':
        output = (
          <div className="text-xs font-mono text-neutral-300">
            rustc 1.85.0 (4d91de4e4 2025-02-18) ({hardware.architecture == 'armv7l' ? 'armv7-unknown-linux-gnueabihf' : hardware.architecture == 'aarch64' ? 'aarch64-unknown-linux-gnu' : 'x86_64-unknown-linux-gnu'})
          </div>
        );
        break;

      case 'cargo':
        if (args.join(' ').includes('build')) {
          output = (
            <div className="text-xs font-mono space-y-1 text-neutral-300">
              <div className="text-cyan-400">   Compiling smithay v0.3.0</div>
              <div className="text-cyan-400">   Compiling tiny-skia v0.11.4</div>
              <div className="text-cyan-400">   Compiling oxide-config v0.9.4</div>
              <div className="text-cyan-400">   Compiling oxide-compositor v0.9.4</div>
              <div className="text-cyan-400">   Compiling oxide-panel v0.9.4</div>
              <div className="text-cyan-400">   Compiling oxide-launcher v0.9.4</div>
              <div className="text-emerald-400 font-semibold pt-1">
                Finished `release` profile [optimized + lto] in 8.42s
              </div>
              <div className="text-neutral-400 text-[11px]">
                Binary size: oxide-compositor (4.2 MB), oxide-panel (2.1 MB), oxide-launcher (1.8 MB)
              </div>
            </div>
          );
        } else {
          output = <div className="text-xs font-mono text-neutral-400">Cargo package manager for Rust. Try: cargo build --release</div>;
        }
        break;

      case 'oxide-ctl':
        if (args[0] === '--status') {
          output = (
            <div className="text-xs font-mono space-y-1 text-neutral-300">
              <div className="text-amber-400 font-semibold">OxideDE Compositor Telemetry:</div>
              <div>Renderer Backend: <span className="text-cyan-300">{hardware.compositorBackend}</span></div>
              <div>Frame Time: <span className="text-emerald-400">0.82 ms</span> (Budget: 16.6 ms for 60Hz)</div>
              <div>Direct Scanout: {config.lowSpecMode ? 'Bypassing blending for fullscreen surfaces (Active)' : 'Standard'}</div>
              <div>Damage Tracking: <span className="text-emerald-400">Enabled</span> (Reduces CPU cycles by ~70% on ARM)</div>
              <div>Wayland Clients Connected: 4 (panel, launcher, terminal, sysmon)</div>
            </div>
          );
        } else if (args[0] === '--low-spec') {
          const val = args[1]?.toLowerCase();
          if (val === 'on' || val === 'true') {
            onUpdateConfig?.({ lowSpecMode: true });
            output = <div className="text-xs font-mono text-emerald-400">Low-spec mode enabled. Blur disabled, direct scanout activated.</div>;
          } else if (val === 'off' || val === 'false') {
            onUpdateConfig?.({ lowSpecMode: false });
            output = <div className="text-xs font-mono text-amber-400">Low-spec mode disabled. Full composite blending restored.</div>;
          } else {
            output = <div className="text-xs font-mono text-neutral-400">Usage: oxide-ctl --low-spec [on|off]</div>;
          }
        } else {
          output = <div className="text-xs font-mono text-neutral-400">Usage: oxide-ctl [--status | --low-spec on/off]</div>;
        }
        break;

      case 'ls':
        output = (
          <div className="text-xs font-mono text-cyan-300 grid grid-cols-3 gap-2">
            <span>Cargo.toml</span>
            <span>crates/</span>
            <span>scripts/</span>
            <span>packaging/</span>
            <span>wayland-sessions/</span>
            <span>README.md</span>
          </div>
        );
        break;

      case 'cat':
        if (args[0] === 'Cargo.toml') {
          output = (
            <pre className="text-xs font-mono text-neutral-300 overflow-x-auto">
{`[workspace]
members = ["crates/oxide-compositor", "crates/oxide-panel", "crates/oxide-launcher"]
resolver = "2"

[profile.release]
opt-level = 3
lto = "fat"
panic = "abort"
strip = "symbols"`}
            </pre>
          );
        } else {
          output = <div className="text-xs font-mono text-neutral-400">File not found. Try: cat Cargo.toml</div>;
        }
        break;

      default:
        output = (
          <div className="text-xs font-mono text-red-400">
            command not found: {cmd}. Type <span className="text-neutral-200 underline">help</span> for commands.
          </div>
        );
    }

    setHistory((prev) => [
      ...prev,
      {
        id: Math.random().toString(),
        command: cmd,
        output,
      },
    ]);
    setInputVal('');
  };

  return (
    <div className="flex h-full min-h-0 flex-col bg-neutral-950 font-mono text-neutral-100 select-text">
      {/* Terminal Top Info Strip */}
      <div className="flex shrink-0 items-center justify-between border-b border-neutral-800 bg-neutral-900 px-3 py-1.5 text-[11px] text-neutral-400">
        <div className="flex items-center gap-2">
          <TerminalIcon className="w-3.5 h-3.5 text-amber-400" />
          <span>oxide@arm-box: ~</span>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-emerald-400">{hardware.architecture}</span>
          <span className="text-neutral-500">|</span>
          <span className="text-cyan-400">{hardware.compositorBackend.split(' ')[0]}</span>
        </div>
      </div>

      {/* Terminal Scrollback Body */}
      <div className="min-h-0 flex-1 space-y-3 overflow-y-auto p-3 font-mono text-xs">
        {history.map((item) => (
          <div key={item.id} className="space-y-1">
            <div className="flex items-center gap-2 text-neutral-300">
              <span className="text-amber-400 font-bold select-none">oxide@arm:~$</span>
              <span className="font-semibold text-white">{item.command}</span>
            </div>
            <div className="pl-2">{item.output}</div>
          </div>
        ))}
        <div ref={terminalEndRef} />
      </div>

      {/* Input prompt */}
      <form onSubmit={handleCommand} className="flex shrink-0 items-center gap-2 border-t border-neutral-800 bg-neutral-900/60 px-3 py-2">
        <span className="text-amber-400 font-bold text-xs select-none">oxide@arm:~$</span>
        <input
          type="text"
          value={inputVal}
          onChange={(e) => setInputVal(e.target.value)}
          placeholder="Type 'help', 'fastfetch', 'free -m', 'cargo build --release'..."
          className="flex-1 bg-transparent text-white text-xs outline-none font-mono placeholder:text-neutral-600"
          autoFocus
        />
        <button
          type="submit"
          className="p-1.5 bg-neutral-800 hover:bg-neutral-700 text-neutral-300 hover:text-white rounded transition-colors flex items-center justify-center shrink-0"
          title="Run command (Enter)"
          aria-label="Run command"
        >
          <CornerDownLeft className="w-3.5 h-3.5" />
        </button>
      </form>
    </div>
  );
}
