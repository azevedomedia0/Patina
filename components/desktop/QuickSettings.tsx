'use client';

import React from 'react';
import { HardwareProfile, DEConfig, ArchType, HARDWARE_PROFILES, AppId } from '@/lib/oxide-types';
import { Sliders, Zap, Monitor, Layout, Palette, Volume2, Cpu, Activity, X } from 'lucide-react';

interface QuickSettingsProps {
  isOpen: boolean;
  onClose: () => void;
  hardware: HardwareProfile;
  config: DEConfig;
  onUpdateConfig: (partial: Partial<DEConfig>) => void;
  onSelectHardware: (arch: ArchType) => void;
  onOpenApp: (appId: AppId) => void;
}

export default function QuickSettings({
  isOpen,
  onClose,
  hardware,
  config,
  onUpdateConfig,
  onSelectHardware,
  onOpenApp,
}: QuickSettingsProps) {
  if (!isOpen) return null;

  const theme = config.activeTheme;
  const opts = config.optimizations;

  return (
    <div
      style={{
        backgroundColor: theme.palette.surface,
        borderColor: theme.palette.border,
        color: theme.palette.textPrimary,
        borderRadius: `${Math.max(8, theme.windowStyle.borderRadius)}px`,
        fontFamily: theme.fontFamily === 'System Sans' ? 'inherit' : theme.fontFamily,
      }}
      className="absolute top-10 right-2 w-84 max-h-[88vh] backdrop-blur-xl border shadow-2xl p-3.5 flex flex-col space-y-3.5 z-50 text-xs select-none overflow-y-auto"
    >
      {/* Header */}
      <div className="flex items-center justify-between pb-2 border-b" style={{ borderColor: theme.palette.border }}>
        <div className="flex items-center gap-2">
          <Sliders className="w-4 h-4" style={{ color: theme.palette.accent }} />
          <span className="font-semibold text-white">Oxide Quick Settings</span>
        </div>
        <button
          onClick={onClose}
          style={{ color: theme.palette.textSecondary }}
          className="hover:text-white p-1 rounded hover:bg-neutral-800 transition-colors"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* Quick App Shortcuts for Theme Studio & Profiler */}
      <div className="grid grid-cols-2 gap-2">
        <button
          onClick={() => {
            onClose();
            onOpenApp('theme-studio');
          }}
          style={{
            backgroundColor: theme.palette.panel,
            borderColor: theme.palette.border,
          }}
          className="p-2.5 rounded-lg border text-left flex items-center gap-2 hover:border-amber-500 transition-colors"
        >
          <Palette className="w-4 h-4 text-purple-400 shrink-0" />
          <div>
            <div className="font-semibold text-xs text-white">Theme Studio</div>
            <div className="text-[10px]" style={{ color: theme.palette.textSecondary }}>Palettes & Fonts</div>
          </div>
        </button>

        <button
          onClick={() => {
            onClose();
            onOpenApp('profiler');
          }}
          style={{
            backgroundColor: theme.palette.panel,
            borderColor: theme.palette.border,
          }}
          className="p-2.5 rounded-lg border text-left flex items-center gap-2 hover:border-cyan-500 transition-colors"
        >
          <Activity className="w-4 h-4 text-cyan-400 shrink-0" />
          <div>
            <div className="font-semibold text-xs text-white">ARM Profiler</div>
            <div className="text-[10px]" style={{ color: theme.palette.textSecondary }}>Microsecond latency</div>
          </div>
        </button>
      </div>

      {/* Quick Toggles Grid */}
      <div className="grid grid-cols-2 gap-2">
        {/* Low-Spec Hardware Mode */}
        <button
          onClick={() => onUpdateConfig({ lowSpecMode: !config.lowSpecMode })}
          style={{
            backgroundColor: config.lowSpecMode ? `${theme.palette.success}20` : theme.palette.background,
            borderColor: config.lowSpecMode ? theme.palette.success : theme.palette.border,
          }}
          className="p-2.5 rounded-lg border text-left flex flex-col justify-between h-18 transition-all"
        >
          <div className="flex items-center justify-between w-full">
            <Zap className="w-4 h-4" style={{ color: config.lowSpecMode ? theme.palette.success : theme.palette.textSecondary }} />
            <span className="text-[10px] font-mono uppercase font-bold" style={{ color: config.lowSpecMode ? theme.palette.success : theme.palette.textSecondary }}>
              {config.lowSpecMode ? 'ON' : 'OFF'}
            </span>
          </div>
          <div>
            <div className="font-semibold text-xs text-white">Low-Spec Mode</div>
            <div className="text-[10px]" style={{ color: theme.palette.textSecondary }}>Software blit fallback</div>
          </div>
        </button>

        {/* Tiling Window Mode */}
        <button
          onClick={() => onUpdateConfig({ tilingMode: !config.tilingMode })}
          style={{
            backgroundColor: config.tilingMode ? `${theme.palette.accent}20` : theme.palette.background,
            borderColor: config.tilingMode ? theme.palette.accent : theme.palette.border,
          }}
          className="p-2.5 rounded-lg border text-left flex flex-col justify-between h-18 transition-all"
        >
          <div className="flex items-center justify-between w-full">
            <Layout className="w-4 h-4" style={{ color: config.tilingMode ? theme.palette.accent : theme.palette.textSecondary }} />
            <span className="text-[10px] font-mono uppercase font-bold" style={{ color: config.tilingMode ? theme.palette.accent : theme.palette.textSecondary }}>
              {config.tilingMode ? 'TILING' : 'FLOAT'}
            </span>
          </div>
          <div>
            <div className="font-semibold text-xs text-white">Window Layout</div>
            <div className="text-[10px]" style={{ color: theme.palette.textSecondary }}>Automatic BSP</div>
          </div>
        </button>
      </div>

      {/* ARM Core Optimizations Quick Strip */}
      <div
        style={{
          backgroundColor: theme.palette.background,
          borderColor: theme.palette.border,
        }}
        className="p-2.5 rounded-lg border space-y-2"
      >
        <div className="flex items-center justify-between">
          <span className="font-semibold text-xs text-white flex items-center gap-1.5">
            <Cpu className="w-3.5 h-3.5 text-cyan-400" />
            <span>ARM Optimizations</span>
          </span>
          <span className="text-[10px] font-mono text-emerald-400">Active</span>
        </div>

        <div className="grid grid-cols-2 gap-1.5 text-[11px] font-mono">
          <button
            onClick={() =>
              onUpdateConfig({
                optimizations: { ...opts, neonSimd: !opts.neonSimd },
              })
            }
            className={`px-2 py-1 rounded text-center transition-colors ${
              opts.neonSimd ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/40' : 'bg-neutral-900 text-neutral-400'
            }`}
          >
            NEON SIMD: {opts.neonSimd ? 'ON' : 'OFF'}
          </button>

          <button
            onClick={() =>
              onUpdateConfig({
                optimizations: { ...opts, kmsHardwarePlanes: !opts.kmsHardwarePlanes },
              })
            }
            className={`px-2 py-1 rounded text-center transition-colors ${
              opts.kmsHardwarePlanes ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40' : 'bg-neutral-900 text-neutral-400'
            }`}
          >
            KMS Plane: {opts.kmsHardwarePlanes ? 'ON' : 'OFF'}
          </button>
        </div>
      </div>

      {/* Target Hardware Architecture Selector */}
      <div className="space-y-1.5">
        <div className="text-[11px] font-semibold flex items-center justify-between" style={{ color: theme.palette.textPrimary }}>
          <span>Target Architecture</span>
          <span className="font-mono text-emerald-400 text-[10px]">{hardware.architecture}</span>
        </div>
        <div className="grid grid-cols-2 gap-1.5">
          {(Object.keys(HARDWARE_PROFILES) as ArchType[]).map((archKey) => {
            const prof = HARDWARE_PROFILES[archKey];
            const isSelected = hardware.id === archKey;
            return (
              <button
                key={archKey}
                onClick={() => onSelectHardware(archKey)}
                style={{
                  backgroundColor: isSelected ? theme.palette.panel : theme.palette.background,
                  borderColor: isSelected ? theme.palette.accent : theme.palette.border,
                }}
                className="p-2 rounded border text-left transition-colors"
              >
                <div className="font-medium text-[11px] truncate text-white">{prof.name.split('(')[0]}</div>
                <div className="text-[10px] font-mono" style={{ color: theme.palette.textSecondary }}>{prof.bits}</div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Audio Volume */}
      <div
        style={{
          backgroundColor: theme.palette.background,
          borderColor: theme.palette.border,
        }}
        className="space-y-1 p-2 rounded-lg border"
      >
        <div className="flex items-center justify-between text-[11px]" style={{ color: theme.palette.textSecondary }}>
          <span className="flex items-center gap-1.5">
            <Volume2 className="w-3.5 h-3.5" />
            <span>Volume (PipeWire)</span>
          </span>
          <span className="font-mono text-neutral-300">75%</span>
        </div>
        <input
          type="range"
          min="0"
          max="100"
          defaultValue="75"
          className="w-full accent-amber-500 h-1 bg-neutral-800 rounded-lg cursor-pointer"
        />
      </div>

      {/* Bottom Button: Open Full Settings */}
      <div className="pt-1 flex items-center gap-2">
        <button
          onClick={() => {
            onClose();
            onOpenApp('settings');
          }}
          style={{
            backgroundColor: theme.palette.panel,
            borderColor: theme.palette.border,
          }}
          className="w-full py-1.5 hover:bg-neutral-800 text-white font-medium rounded text-xs transition-colors border"
        >
          All System Settings...
        </button>
      </div>
    </div>
  );
}
