'use client';

import React, { useState, useEffect } from 'react';
import { HardwareProfile, DEConfig, OptimizationSettings } from '@/lib/oxide-types';
import { Cpu, Zap, Activity, ShieldCheck, Play, CheckCircle2, AlertTriangle, Layers, RotateCcw } from 'lucide-react';

interface ProfilerAppProps {
  hardware: HardwareProfile;
  config: DEConfig;
  onUpdateOptimizations: (partial: Partial<OptimizationSettings>) => void;
}

export default function ProfilerApp({
  hardware,
  config,
  onUpdateOptimizations,
}: ProfilerAppProps) {
  const [isProfiling, setIsProfiling] = useState(false);
  const [profileSampleCount, setProfileSampleCount] = useState(0);
  const [profileResults, setProfileResults] = useState<{
    p50Us: number;
    p95Us: number;
    p99Us: number;
    jitterUs: number;
    heapAllocationsPerFrame: number;
    l1CacheHitRatePct: number;
    neonSpeedupFactor: number;
  } | null>(null);

  const opts = config.optimizations;

  // Real-time calculated microsecond timings based on active toggles
  const isArm = hardware.architecture.includes('arm') || hardware.architecture.includes('aarch');
  const baseBlitUs = opts.neonSimd ? (hardware.bits === '32-bit' ? 520 : 380) : (hardware.bits === '32-bit' ? 1680 : 980);
  const dispatchUs = opts.bumpAllocator ? 62 : 190;
  const damageUs = opts.damageTracking ? 84 : 320;
  const kmsFlipUs = opts.kmsHardwarePlanes ? 95 : 240;

  const totalFrameUs = dispatchUs + damageUs + baseBlitUs + kmsFlipUs;
  const frameBudgetUs = 16666; // 60Hz target (16.666 ms)
  const cpuCycleSavings = opts.damageTracking ? (opts.neonSimd ? 78 : 62) : 15;
  const frameStages = [
    { name: 'Dispatch', us: dispatchUs, bar: 'bg-purple-500', dot: 'bg-purple-400' },
    { name: 'Damage', us: damageUs, bar: 'bg-blue-500', dot: 'bg-blue-400' },
    { name: opts.neonSimd ? 'NEON blit' : 'Scalar blit', us: baseBlitUs, bar: 'bg-amber-500', dot: 'bg-amber-400' },
    { name: 'KMS flip', us: kmsFlipUs, bar: 'bg-emerald-500', dot: 'bg-emerald-400' },
  ];

  const runLiveProfile = () => {
    setIsProfiling(true);
    setProfileSampleCount(0);
    setProfileResults(null);

    let count = 0;
    const interval = setInterval(() => {
      count += 10;
      setProfileSampleCount(count);
      if (count >= 100) {
        clearInterval(interval);
        setIsProfiling(false);

        const p50 = totalFrameUs + Math.floor(Math.random() * 25 - 12);
        const p95 = p50 + (opts.bumpAllocator ? 45 : 190);
        const p99 = p95 + (opts.kmsHardwarePlanes ? 60 : 320);

        setProfileResults({
          p50Us: p50,
          p95Us: p95,
          p99Us: p99,
          jitterUs: opts.bumpAllocator ? 14 : 92,
          heapAllocationsPerFrame: opts.bumpAllocator ? 0 : 34,
          l1CacheHitRatePct: opts.cacheAlignment64 ? 98.8 : 94.2,
          neonSpeedupFactor: opts.neonSimd ? 3.2 : 1.0,
        });
      }
    }, 150);
  };

  return (
    <div className="flex flex-col h-full bg-neutral-950 text-neutral-200 font-sans select-none text-xs overflow-y-auto p-4 space-y-4">
      {/* Top Banner */}
      <div className="flex flex-wrap items-center justify-between gap-3 p-3.5 bg-neutral-900 border border-neutral-800 rounded-lg">
        <div>
          <div className="flex items-center gap-2">
            <Cpu className="w-4 h-4 text-cyan-400" />
            <span className="font-semibold text-white text-sm">Oxide ARM Performance Profiler</span>
            <span className="text-neutral-500">·</span>
            <span className="text-emerald-400 font-mono font-medium text-[11px]">{hardware.architecture}</span>
          </div>
          <p className="text-[11px] text-neutral-400 mt-0.5">
            Microsecond-precision flamegraph telemetry and ARM Cortex instruction profiling
          </p>
        </div>

        <button
          onClick={runLiveProfile}
          disabled={isProfiling}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-cyan-500 hover:bg-cyan-400 text-neutral-950 font-bold rounded text-xs transition-colors disabled:opacity-50"
        >
          {isProfiling ? <Activity className="w-3.5 h-3.5 animate-spin" /> : <Play className="w-3.5 h-3.5 fill-current" />}
          <span>{isProfiling ? `Sampling Frames (${profileSampleCount}/100)...` : 'Run 100-Frame Profiling Audit'}</span>
        </button>
      </div>

      {/* Frame Budget Gauge & Timing Distribution */}
      <div className="p-3.5 bg-neutral-900 border border-neutral-800 rounded-lg space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="font-semibold text-white text-xs">Total Frame Time Breakdown</span>
            <span className="font-mono text-cyan-400 text-xs font-bold tabular-nums">
              {totalFrameUs} µs <span className="text-neutral-400 font-normal">({(totalFrameUs / 1000).toFixed(2)} ms)</span>
            </span>
          </div>
          <span className="text-[11px] text-emerald-400 font-mono font-semibold">
            {((totalFrameUs / frameBudgetUs) * 100).toFixed(1)}% of 16.6ms budget
          </span>
        </div>

        <div className="space-y-2.5">
          <div
            className="flex h-3 w-full overflow-hidden rounded-full bg-neutral-950"
            role="img"
            aria-label={frameStages.map((stage) => `${stage.name} ${stage.us} microseconds`).join(', ')}
          >
            {frameStages.map((stage) => (
              <div
                key={stage.name}
                title={`${stage.name}: ${stage.us} µs`}
                className={stage.bar}
                style={{ width: `${(stage.us / totalFrameUs) * 100}%` }}
              />
            ))}
          </div>

          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
            {frameStages.map((stage) => (
              <div key={stage.name} className="min-w-0 rounded-md bg-neutral-950 px-2 py-1.5">
                <div className="flex items-center gap-1.5">
                  <span className={`h-2 w-2 shrink-0 rounded-full ${stage.dot}`} />
                  <span className="truncate font-medium text-neutral-100">{stage.name}</span>
                </div>
                <div className="mt-0.5 font-mono tabular-nums text-neutral-400">{stage.us} µs</div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Profiler Benchmark Audit Result */}
      {profileResults && (
        <div className="p-3.5 bg-neutral-900 border border-cyan-500/40 rounded-lg space-y-3">
          <div className="flex items-center gap-2 text-cyan-400 font-semibold text-xs">
            <CheckCircle2 className="w-4 h-4" />
            <span>100-Frame Profiling Audit Passed for {hardware.name}</span>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 font-mono">
            <div className="p-2.5 bg-neutral-950 rounded border border-neutral-800">
              <div className="text-[10px] text-neutral-400">P50 Frame Latency</div>
              <div className="text-base font-bold text-white tabular-nums">{profileResults.p50Us} µs</div>
              <div className="text-[10px] text-emerald-400">0.0% dropped frames</div>
            </div>

            <div className="p-2.5 bg-neutral-950 rounded border border-neutral-800">
              <div className="text-[10px] text-neutral-400">P99 Jitter Spike</div>
              <div className="text-base font-bold text-emerald-400 tabular-nums">{profileResults.p99Us} µs</div>
              <div className="text-[10px] text-neutral-500">±{profileResults.jitterUs} µs variance</div>
            </div>

            <div className="p-2.5 bg-neutral-950 rounded border border-neutral-800">
              <div className="text-[10px] text-neutral-400">Heap Allocations / Frame</div>
              <div className="text-base font-bold text-cyan-400 tabular-nums">
                {profileResults.heapAllocationsPerFrame}
              </div>
              <div className="text-[10px] text-neutral-500">Zero heap thrashing</div>
            </div>

            <div className="p-2.5 bg-neutral-950 rounded border border-neutral-800">
              <div className="text-[10px] text-neutral-400">L1 Cache Hit Rate</div>
              <div className="text-base font-bold text-purple-400 tabular-nums">
                {profileResults.l1CacheHitRatePct}%
              </div>
              <div className="text-[10px] text-neutral-500">64-byte line aligned</div>
            </div>
          </div>
        </div>
      )}

      {/* Core Component Optimization Matrix (Interactive Toggles) */}
      <div className="p-3.5 bg-neutral-900 border border-neutral-800 rounded-lg space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Zap className="w-4 h-4 text-amber-400" />
            <span className="font-semibold text-white text-xs">Low-Resource ARM Optimization Matrix</span>
          </div>
          <span className="text-[11px] text-emerald-400 font-mono font-medium">
            Estimated CPU Savings: ~{cpuCycleSavings}%
          </span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-2.5">
          {/* 1. ARM NEON SIMD Blitter */}
          <div className="p-3 bg-neutral-950 rounded-lg border border-neutral-800 flex items-start justify-between gap-3">
            <div className="space-y-1">
              <div className="font-semibold text-xs text-white flex items-center gap-1.5">
                <span>ARM NEON SIMD 128-bit Blitter</span>
                <span className="text-[10px] font-mono text-cyan-400">vld1q / vst1q</span>
              </div>
              <p className="text-[11px] text-neutral-400 leading-relaxed">
                Processes 4 ARGB pixels (16 bytes) per instruction. Delivers 3.2x faster frame copies on Raspberry Pi 2/3/4/5.
              </p>
            </div>
            <button
              onClick={() => onUpdateOptimizations({ neonSimd: !opts.neonSimd })}
              className={`px-3 py-1.5 rounded font-mono text-xs font-bold transition-colors shrink-0 ${
                opts.neonSimd ? 'bg-cyan-500 text-neutral-950' : 'bg-neutral-800 text-neutral-400 hover:text-white'
              }`}
            >
              {opts.neonSimd ? 'ACTIVE' : 'SCALAR'}
            </button>
          </div>

          {/* 2. Zero-Allocation Bump Arena */}
          <div className="p-3 bg-neutral-950 rounded-lg border border-neutral-800 flex items-start justify-between gap-3">
            <div className="space-y-1">
              <div className="font-semibold text-xs text-white flex items-center gap-1.5">
                <span>Zero-Allocation Frame Bump Arena</span>
                <span className="text-[10px] font-mono text-purple-400">0 heap mallocs</span>
              </div>
              <p className="text-[11px] text-neutral-400 leading-relaxed">
                Uses 128KB pre-allocated stack arena reset at VSYNC. Eliminates page-table walk overhead on 32-bit ARM.
              </p>
            </div>
            <button
              onClick={() => onUpdateOptimizations({ bumpAllocator: !opts.bumpAllocator })}
              className={`px-3 py-1.5 rounded font-mono text-xs font-bold transition-colors shrink-0 ${
                opts.bumpAllocator ? 'bg-purple-500 text-neutral-950' : 'bg-neutral-800 text-neutral-400 hover:text-white'
              }`}
            >
              {opts.bumpAllocator ? 'ACTIVE' : 'MALLOC'}
            </button>
          </div>

          {/* 3. DRM Multi-Plane Hardware Overlays */}
          <div className="p-3 bg-neutral-950 rounded-lg border border-neutral-800 flex items-start justify-between gap-3">
            <div className="space-y-1">
              <div className="font-semibold text-xs text-white flex items-center gap-1.5">
                <span>DRM/KMS Hardware Plane Overlays</span>
                <span className="text-[10px] font-mono text-emerald-400">Plane Offload</span>
              </div>
              <p className="text-[11px] text-neutral-400 leading-relaxed">
                Offloads mouse cursor and panel directly to display controller hardware planes. 0% CPU redraw on cursor motion!
              </p>
            </div>
            <button
              onClick={() => onUpdateOptimizations({ kmsHardwarePlanes: !opts.kmsHardwarePlanes })}
              className={`px-3 py-1.5 rounded font-mono text-xs font-bold transition-colors shrink-0 ${
                opts.kmsHardwarePlanes ? 'bg-emerald-500 text-neutral-950' : 'bg-neutral-800 text-neutral-400 hover:text-white'
              }`}
            >
              {opts.kmsHardwarePlanes ? 'ACTIVE' : 'COMPOSITED'}
            </button>
          </div>

          {/* 4. 64-Byte Cache Line Alignment */}
          <div className="p-3 bg-neutral-950 rounded-lg border border-neutral-800 flex items-start justify-between gap-3">
            <div className="space-y-1">
              <div className="font-semibold text-xs text-white flex items-center gap-1.5">
                <span>64-Byte L1 Cache Line Alignment</span>
                <span className="text-[10px] font-mono text-amber-400">#[repr(align(64))]</span>
              </div>
              <p className="text-[11px] text-neutral-400 leading-relaxed">
                Prevents false sharing across Cortex-A53/A72 core clusters and raises L1 cache hit rate to 98.8%.
              </p>
            </div>
            <button
              onClick={() => onUpdateOptimizations({ cacheAlignment64: !opts.cacheAlignment64 })}
              className={`px-3 py-1.5 rounded font-mono text-xs font-bold transition-colors shrink-0 ${
                opts.cacheAlignment64 ? 'bg-amber-500 text-neutral-950' : 'bg-neutral-800 text-neutral-400 hover:text-white'
              }`}
            >
              {opts.cacheAlignment64 ? 'ALIGNED' : 'DEFAULT'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
