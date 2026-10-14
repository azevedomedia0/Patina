'use client';

import React, { useState, useEffect } from 'react';
import { HardwareProfile, DEConfig } from '@/lib/oxide-types';
import { Cpu, HardDrive, Activity, Gauge, Zap, CheckCircle2, Sliders } from 'lucide-react';

interface SystemMonitorAppProps {
  hardware: HardwareProfile;
  config: DEConfig;
  onToggleLowSpec?: () => void;
}

interface ProcessItem {
  pid: number;
  name: string;
  user: string;
  cpu: number;
  memMB: number;
  threads: number;
  state: 'Running' | 'Sleeping';
}

export default function SystemMonitorApp({ hardware, config, onToggleLowSpec }: SystemMonitorAppProps) {
  const [coreLoads, setCoreLoads] = useState<number[]>([12, 8, 15, 6]);
  const [frameTimeMs, setFrameTimeMs] = useState(0.84);
  const [tick, setTick] = useState(0);

  // Dynamic process list
  const [processes, setProcesses] = useState<ProcessItem[]>([
    { pid: 1042, name: 'oxide-compositor', user: 'oxide', cpu: 1.8, memMB: 18.4, threads: 4, state: 'Running' },
    { pid: 1048, name: 'oxide-panel', user: 'oxide', cpu: 0.4, memMB: 7.2, threads: 2, state: 'Sleeping' },
    { pid: 1055, name: 'oxide-launcher', user: 'oxide', cpu: 0.1, memMB: 4.1, threads: 2, state: 'Sleeping' },
    { pid: 1089, name: 'pipewire', user: 'oxide', cpu: 0.3, memMB: 3.8, threads: 2, state: 'Sleeping' },
    { pid: 1092, name: 'wireplumber', user: 'oxide', cpu: 0.2, memMB: 4.6, threads: 2, state: 'Sleeping' },
    { pid: 1104, name: 'udev-input-rs', user: 'root', cpu: 0.2, memMB: 2.1, threads: 1, state: 'Sleeping' },
    { pid: 1120, name: 'dbus-daemon', user: 'messagebus', cpu: 0.1, memMB: 2.4, threads: 1, state: 'Sleeping' },
  ]);

  useEffect(() => {
    const interval = setInterval(() => {
      setTick((t) => t + 1);
      // Realistic fluctuation based on hardware
      const baseLoad = hardware.bits === '32-bit' ? 14 : 7;
      setCoreLoads((loads) =>
        loads.map(() => Math.min(100, Math.max(2, Math.round(baseLoad + Math.sin(Date.now() / 800 + Math.random()) * 8))))
      );
      setFrameTimeMs(+(0.75 + Math.random() * 0.25).toFixed(2));
    }, 1500);

    return () => clearInterval(interval);
  }, [hardware.bits]);

  const activeRamMB = config.lowSpecMode ? hardware.idleRamMB - 6 : hardware.idleRamMB;
  const ramPercent = ((activeRamMB / hardware.ramTotalMB) * 100).toFixed(1);
  const avgCpu = Math.round(coreLoads.reduce((a, b) => a + b, 0) / coreLoads.length);

  return (
    <div className="flex h-full min-h-0 flex-col space-y-4 overflow-y-auto bg-neutral-950 p-4 font-sans text-xs text-neutral-200 select-none">
      {/* Top Banner: Architecture & Active Compositor Pipeline */}
      <div className="flex flex-wrap items-center justify-between gap-3 p-3 rounded-lg bg-neutral-900 border border-neutral-800">
        <div>
          <div className="flex items-center gap-2">
            <span className="font-semibold text-white text-sm">{hardware.name}</span>
            <span className="text-[11px] text-neutral-400">·</span>
            <span className="text-emerald-400 font-mono font-medium">{hardware.architecture}</span>
            <span className="text-[11px] text-neutral-400">·</span>
            <span className="text-neutral-400">{hardware.bits}</span>
          </div>
          <div className="text-[11px] text-neutral-400 mt-0.5">
            Driver: <span className="text-neutral-300">{hardware.gpuDriver}</span>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <div className="text-right">
            <div className="text-[10px] text-neutral-400 uppercase tracking-wider">Compositor Backend</div>
            <div className="text-cyan-400 font-mono font-medium">{hardware.compositorBackend}</div>
          </div>
          <button
            onClick={onToggleLowSpec}
            className={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
              config.lowSpecMode
                ? 'bg-emerald-600/30 text-emerald-300 border border-emerald-500/40 hover:bg-emerald-600/40'
                : 'bg-neutral-800 text-neutral-300 border border-neutral-700 hover:bg-neutral-700'
            }`}
          >
            {config.lowSpecMode ? 'Low-Spec Mode: ON' : 'Low-Spec Mode: OFF'}
          </button>
        </div>
      </div>

      {/* Main KPI Stat Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
        {/* RAM Metric Card */}
        <div className="p-3 rounded-lg bg-neutral-900/90 border border-neutral-800 space-y-2">
          <div className="flex items-center justify-between text-neutral-400">
            <span className="flex items-center gap-1.5 font-medium">
              <HardDrive className="w-3.5 h-3.5 text-cyan-400" />
              Memory (RSS)
            </span>
            <span className="font-mono text-emerald-400 font-semibold tabular-nums">{ramPercent}%</span>
          </div>
          <div className="flex items-baseline gap-1">
            <span className="text-2xl font-bold font-mono text-white tabular-nums">{activeRamMB}</span>
            <span className="text-neutral-400 font-mono">/ {hardware.ramTotalMB} MiB</span>
          </div>
          {/* Progress bar */}
          <div className="w-full bg-neutral-800 h-1.5 rounded-full overflow-hidden">
            <div
              className="bg-emerald-500 h-full transition-all duration-500 rounded-full"
              style={{ width: `${Math.max(4, Math.min(100, (activeRamMB / hardware.ramTotalMB) * 100))}%` }}
            />
          </div>
          <div className="text-[11px] text-neutral-400 flex items-center gap-1 pt-1">
            <CheckCircle2 className="w-3 h-3 text-emerald-400 shrink-0" />
            <span>Sub-35MB idle footprint via Rust memory safety</span>
          </div>
        </div>

        {/* CPU Load Metric Card */}
        <div className="p-3 rounded-lg bg-neutral-900/90 border border-neutral-800 space-y-2">
          <div className="flex items-center justify-between text-neutral-400">
            <span className="flex items-center gap-1.5 font-medium">
              <Cpu className="w-3.5 h-3.5 text-amber-400" />
              CPU Load ({hardware.cpuCores} Cores)
            </span>
            <span className="font-mono text-amber-400 font-semibold tabular-nums">{avgCpu}%</span>
          </div>
          <div className="flex items-baseline gap-1">
            <span className="text-2xl font-bold font-mono text-white tabular-nums">{avgCpu}%</span>
            <span className="text-neutral-400 text-xs">utilization</span>
          </div>
          {/* Core bars */}
          <div className="grid grid-cols-4 gap-1.5 pt-0.5">
            {coreLoads.slice(0, hardware.cpuCores).map((load, idx) => (
              <div key={idx} className="space-y-1">
                <div className="h-10 bg-neutral-800 rounded flex flex-col justify-end p-0.5">
                  <div
                    className="w-full bg-amber-500/80 rounded transition-all duration-500"
                    style={{ height: `${load}%` }}
                  />
                </div>
                <div className="text-[9px] text-center text-neutral-400 font-mono">C{idx}</div>
              </div>
            ))}
          </div>
        </div>

        {/* Compositor Engine Card */}
        <div className="p-3 rounded-lg bg-neutral-900/90 border border-neutral-800 space-y-2">
          <div className="flex items-center justify-between text-neutral-400">
            <span className="flex items-center gap-1.5 font-medium">
              <Gauge className="w-3.5 h-3.5 text-emerald-400" />
              Compositor Frame Budget
            </span>
            <span className="font-mono text-emerald-400 font-semibold tabular-nums">60 FPS</span>
          </div>
          <div className="flex items-baseline gap-1">
            <span className="text-2xl font-bold font-mono text-white tabular-nums">{frameTimeMs}</span>
            <span className="text-neutral-400 font-mono">ms / 16.6ms</span>
          </div>
          <div className="w-full bg-neutral-800 h-1.5 rounded-full overflow-hidden">
            <div
              className="bg-cyan-500 h-full rounded-full transition-all duration-300"
              style={{ width: `${(frameTimeMs / 16.6) * 100}%` }}
            />
          </div>
          <div className="text-[11px] text-neutral-400 pt-1">
            Damage tracking: <span className="text-emerald-400">Active</span> · Redraw savings: <span className="text-white font-mono">~74%</span>
          </div>
        </div>
      </div>

      {/* Memory Comparison: OxideDE vs Other Linux Desktop Environments */}
      <div className="p-3.5 rounded-lg bg-neutral-900 border border-neutral-800 space-y-2.5">
        <div className="flex items-center justify-between">
          <span className="font-semibold text-white text-xs">Idle Memory Consumption Benchmark (ARM & x86)</span>
          <span className="text-[11px] text-neutral-400">Lower is better</span>
        </div>

        <div className="space-y-2 pt-1">
          {/* OxideDE */}
          <div className="space-y-1">
            <div className="flex items-center justify-between text-[11px]">
              <span className="font-semibold text-emerald-400 flex items-center gap-1.5">
                ★ OxideDE (Rust + Smithay)
                <span className="text-[10px] text-emerald-500/80">({hardware.architecture})</span>
              </span>
              <span className="font-mono font-bold text-emerald-300 tabular-nums">{activeRamMB} MiB</span>
            </div>
            <div className="w-full bg-neutral-800 h-2 rounded-full overflow-hidden">
              <div className="bg-emerald-500 h-full rounded-full" style={{ width: `${(activeRamMB / 650) * 100}%` }} />
            </div>
          </div>

          {/* LXQt */}
          <div className="space-y-1">
            <div className="flex items-center justify-between text-[11px] text-neutral-400">
              <span>LXQt (C++ / Qt)</span>
              <span className="font-mono tabular-nums">142 MiB</span>
            </div>
            <div className="w-full bg-neutral-800 h-1.5 rounded-full overflow-hidden">
              <div className="bg-neutral-600 h-full rounded-full" style={{ width: `${(142 / 650) * 100}%` }} />
            </div>
          </div>

          {/* XFCE */}
          <div className="space-y-1">
            <div className="flex items-center justify-between text-[11px] text-neutral-400">
              <span>XFCE 4.18 (C / GTK3)</span>
              <span className="font-mono tabular-nums">265 MiB</span>
            </div>
            <div className="w-full bg-neutral-800 h-1.5 rounded-full overflow-hidden">
              <div className="bg-neutral-600 h-full rounded-full" style={{ width: `${(265 / 650) * 100}%` }} />
            </div>
          </div>

          {/* KDE Plasma */}
          <div className="space-y-1">
            <div className="flex items-center justify-between text-[11px] text-neutral-400">
              <span>KDE Plasma 6 (C++ / Qt6)</span>
              <span className="font-mono tabular-nums">480 MiB</span>
            </div>
            <div className="w-full bg-neutral-800 h-1.5 rounded-full overflow-hidden">
              <div className="bg-neutral-600 h-full rounded-full" style={{ width: `${(480 / 650) * 100}%` }} />
            </div>
          </div>

          {/* GNOME */}
          <div className="space-y-1">
            <div className="flex items-center justify-between text-[11px] text-neutral-400">
              <span>GNOME 46 (C / GJS / GTK4)</span>
              <span className="font-mono text-red-400 tabular-nums">640 MiB</span>
            </div>
            <div className="w-full bg-neutral-800 h-1.5 rounded-full overflow-hidden">
              <div className="bg-red-500/70 h-full rounded-full" style={{ width: `${(640 / 650) * 100}%` }} />
            </div>
          </div>
        </div>
      </div>

      {/* Process Table */}
      <div className="p-3.5 rounded-lg bg-neutral-900 border border-neutral-800 space-y-2">
        <div className="flex items-center justify-between">
          <span className="font-semibold text-white text-xs">Active System Daemons & Processes</span>
          <span className="text-[11px] text-neutral-400 font-mono">{processes.length} running</span>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left font-mono text-[11px]">
            <thead>
              <tr className="border-b border-neutral-800 text-neutral-400 text-[10px]">
                <th className="py-1.5 font-normal">PID</th>
                <th className="py-1.5 font-normal">Process</th>
                <th className="py-1.5 font-normal">User</th>
                <th className="py-1.5 font-normal text-right">CPU%</th>
                <th className="py-1.5 font-normal text-right">Memory</th>
                <th className="py-1.5 font-normal text-right">Threads</th>
                <th className="py-1.5 font-normal text-right">State</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-neutral-850">
              {processes.map((proc) => (
                <tr key={proc.pid} className="hover:bg-neutral-800/40">
                  <td className="py-1.5 text-neutral-400">{proc.pid}</td>
                  <td className="py-1.5 text-neutral-200 font-semibold">{proc.name}</td>
                  <td className="py-1.5 text-neutral-400">{proc.user}</td>
                  <td className="py-1.5 text-right text-amber-300 tabular-nums">{proc.cpu.toFixed(1)}%</td>
                  <td className="py-1.5 text-right text-emerald-400 tabular-nums">{proc.memMB.toFixed(1)} MB</td>
                  <td className="py-1.5 text-right text-neutral-400 tabular-nums">{proc.threads}</td>
                  <td className="py-1.5 text-right">
                    <span className="text-[10px] text-neutral-400">
                      {proc.state}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
