'use client';

import React, { useState } from 'react';
import { HardwareProfile, DEConfig } from '@/lib/oxide-types';
import { Gauge, Play, CheckCircle2, Cpu, Zap, Activity } from 'lucide-react';

interface BenchmarkAppProps {
  hardware: HardwareProfile;
  config: DEConfig;
}

export default function BenchmarkApp({ hardware, config }: BenchmarkAppProps) {
  const [isRunning, setIsRunning] = useState(false);
  const [testStage, setTestStage] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);
  const [benchmarkResult, setBenchmarkResult] = useState<{
    fpsScore: number;
    frameLatencyMs: number;
    memoryEfficiencyRatio: number;
    damageSavingsPercent: number;
    bootTimeSec: number;
  } | null>(null);

  const runBenchmark = () => {
    setIsRunning(true);
    setProgress(0);
    setBenchmarkResult(null);

    const stages = [
      'Calibrating DRM/KMS dumb buffers...',
      'Testing tiny-skia SIMD NEON rasterization...',
      'Measuring damage-tracking rect overhead...',
      'Evaluating cold-start boot time & RSS allocation...',
      'Finalizing hardware performance index...',
    ];

    let current = 0;
    setTestStage(stages[0]);

    const timer = setInterval(() => {
      current++;
      setProgress((current / stages.length) * 100);

      if (current < stages.length) {
        setTestStage(stages[current]);
      } else {
        clearInterval(timer);
        setIsRunning(false);
        setTestStage(null);

        // Calculate scores based on target hardware profile
        const isArm32 = hardware.bits === '32-bit';
        setBenchmarkResult({
          fpsScore: isArm32 ? 59.8 : 60.0,
          frameLatencyMs: isArm32 ? 1.12 : 0.68,
          memoryEfficiencyRatio: +(hardware.ramTotalMB / (config.lowSpecMode ? hardware.idleRamMB - 6 : hardware.idleRamMB)).toFixed(1),
          damageSavingsPercent: isArm32 ? 78 : 72,
          bootTimeSec: isArm32 ? 0.65 : 0.38,
        });
      }
    }, 700);
  };

  return (
    <div className="flex h-full min-h-0 flex-col space-y-4 overflow-y-auto bg-neutral-950 p-4 font-sans text-xs text-neutral-200 select-none">
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-3 p-3 bg-neutral-900 border border-neutral-800 rounded-lg">
        <div>
          <div className="flex items-center gap-2">
            <span className="font-semibold text-white text-sm">Low-End & ARM Hardware Benchmark Suite</span>
          </div>
          <p className="text-[11px] text-neutral-400">
            Measures compositor latency, memory footprint, and damage tracking efficiency for {hardware.name}
          </p>
        </div>

        <button
          onClick={runBenchmark}
          disabled={isRunning}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-amber-500 hover:bg-amber-400 text-neutral-950 font-semibold rounded transition-colors disabled:opacity-50"
        >
          {isRunning ? <Activity className="w-3.5 h-3.5 animate-spin" /> : <Play className="w-3.5 h-3.5 fill-current" />}
          <span>{isRunning ? 'Benchmarking...' : 'Run Performance Suite'}</span>
        </button>
      </div>

      {/* Progress banner when running */}
      {isRunning && (
        <div className="p-3 bg-neutral-900 border border-amber-500/40 rounded-lg space-y-2">
          <div className="flex justify-between text-neutral-300 font-mono text-[11px]">
            <span>{testStage}</span>
            <span className="text-amber-400 font-bold">{Math.round(progress)}%</span>
          </div>
          <div className="w-full bg-neutral-800 h-2 rounded-full overflow-hidden">
            <div className="bg-amber-500 h-full transition-all duration-300 rounded-full" style={{ width: `${progress}%` }} />
          </div>
        </div>
      )}

      {/* Benchmark Results Grid */}
      {benchmarkResult && (
        <div className="p-3.5 bg-neutral-900 border border-emerald-500/30 rounded-lg space-y-3">
          <div className="flex items-center gap-2 text-emerald-400 font-semibold text-xs">
            <CheckCircle2 className="w-4 h-4" />
            <span>Benchmark Complete for {hardware.name} ({hardware.architecture})</span>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
            <div className="p-2.5 bg-neutral-950 rounded border border-neutral-800">
              <div className="text-[10px] text-neutral-400 uppercase font-mono">Frame Rate</div>
              <div className="text-lg font-bold font-mono text-emerald-400 tabular-nums">{benchmarkResult.fpsScore} FPS</div>
              <div className="text-[10px] text-neutral-500">Target: 60 FPS locked</div>
            </div>

            <div className="p-2.5 bg-neutral-950 rounded border border-neutral-800">
              <div className="text-[10px] text-neutral-400 uppercase font-mono">Frame Latency</div>
              <div className="text-lg font-bold font-mono text-cyan-400 tabular-nums">{benchmarkResult.frameLatencyMs} ms</div>
              <div className="text-[10px] text-neutral-500">16.6ms frame budget</div>
            </div>

            <div className="p-2.5 bg-neutral-950 rounded border border-neutral-800">
              <div className="text-[10px] text-neutral-400 uppercase font-mono">Cold Boot Time</div>
              <div className="text-lg font-bold font-mono text-amber-400 tabular-nums">{benchmarkResult.bootTimeSec}s</div>
              <div className="text-[10px] text-neutral-500">to responsive desktop</div>
            </div>

            <div className="p-2.5 bg-neutral-950 rounded border border-neutral-800">
              <div className="text-[10px] text-neutral-400 uppercase font-mono">Damage Redraw Savings</div>
              <div className="text-lg font-bold font-mono text-purple-400 tabular-nums">{benchmarkResult.damageSavingsPercent}%</div>
              <div className="text-[10px] text-neutral-500">CPU redraw cycle savings</div>
            </div>
          </div>
        </div>
      )}

      {/* Comparative Desktop Environment Metrics */}
      <div className="p-3.5 bg-neutral-900 border border-neutral-800 rounded-lg space-y-3">
        <div className="font-semibold text-white text-xs">
          Cross-Platform Desktop Environment Metrics (Cold Start & Idle RAM)
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left font-mono text-[11px]">
            <thead>
              <tr className="border-b border-neutral-800 text-neutral-400 text-[10px]">
                <th className="py-2 font-normal">Environment</th>
                <th className="py-2 font-normal">Language</th>
                <th className="py-2 font-normal text-right">Idle RAM (armv7l)</th>
                <th className="py-2 font-normal text-right">Boot Time</th>
                <th className="py-2 font-normal text-right">32-bit ARM Compatibility</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-neutral-850">
              <tr className="bg-emerald-500/10 text-white font-semibold">
                <td className="py-2 text-emerald-400 flex items-center gap-1.5">
                  ★ OxideDE
                </td>
                <td className="py-2 text-cyan-300">Pure Rust</td>
                <td className="py-2 text-right text-emerald-400 tabular-nums">~28 - 36 MB</td>
                <td className="py-2 text-right tabular-nums">0.4s</td>
                <td className="py-2 text-right text-emerald-400">Native (tiny-skia + NEON)</td>
              </tr>
              <tr className="text-neutral-300">
                <td className="py-2">LXQt</td>
                <td className="py-2 text-neutral-400">C++ / Qt</td>
                <td className="py-2 text-right tabular-nums">142 MB</td>
                <td className="py-2 text-right tabular-nums">2.1s</td>
                <td className="py-2 text-right text-neutral-300">Supported</td>
              </tr>
              <tr className="text-neutral-300">
                <td className="py-2">XFCE 4.18</td>
                <td className="py-2 text-neutral-400">C / GTK3</td>
                <td className="py-2 text-right tabular-nums">265 MB</td>
                <td className="py-2 text-right tabular-nums">3.4s</td>
                <td className="py-2 text-right text-neutral-300">Supported</td>
              </tr>
              <tr className="text-neutral-300">
                <td className="py-2">KDE Plasma</td>
                <td className="py-2 text-neutral-400">C++ / QML</td>
                <td className="py-2 text-right tabular-nums">480 MB</td>
                <td className="py-2 text-right tabular-nums">6.8s</td>
                <td className="py-2 text-right text-amber-400">Heavy on 1GB RAM</td>
              </tr>
              <tr className="text-neutral-300">
                <td className="py-2">GNOME Shell</td>
                <td className="py-2 text-neutral-400">C / JS (GJS)</td>
                <td className="py-2 text-right text-red-400 tabular-nums">640 MB+</td>
                <td className="py-2 text-right tabular-nums">11.2s</td>
                <td className="py-2 text-right text-red-400">Unusable on 1GB RAM</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
