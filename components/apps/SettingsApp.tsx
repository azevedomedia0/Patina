'use client';

import React from 'react';
import {
  DEConfig,
  HardwareProfile,
  HARDWARE_PROFILES,
  ArchType,
  PRESET_THEMES,
  DynamicTheme,
  ChromeSize,
} from '@/lib/oxide-types';
import { Settings as SettingsIcon, Palette, Monitor, Zap, Layout, Activity, ShieldCheck } from 'lucide-react';

interface SettingsAppProps {
  config: DEConfig;
  hardware: HardwareProfile;
  onUpdateConfig: (partial: Partial<DEConfig>) => void;
  onSelectHardware: (arch: ArchType) => void;
}

export default function SettingsApp({
  config,
  hardware,
  onUpdateConfig,
  onSelectHardware,
}: SettingsAppProps) {
  const wallpapers: { id: DEConfig['wallpaper']; name: string; desc: string }[] = [
    { id: 'stock', name: 'Stock Wallpaper', desc: 'Default scenic desktop background' },
    { id: 'oxide-mesh', name: 'Oxide Charcoal Mesh', desc: 'Minimalist dark mesh with subtle amber accents' },
    { id: 'nordic-aurora', name: 'Nordic Indigo Aurora', desc: 'Deep indigo gradient with soft cyan rim' },
    { id: 'rust-metal', name: 'Anodized Slate & Rust', desc: 'Sleek industrial metal surfaces' },
    { id: 'minimal-charcoal', name: 'Zero-Overhead Flat Charcoal', desc: 'Pure 0% GPU overhead for 32-bit ARM' },
  ];

  const handleSelectPreset = (key: string) => {
    const th = PRESET_THEMES[key];
    if (th) {
      onUpdateConfig({ activeTheme: th });
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col space-y-5 overflow-y-auto bg-neutral-950 p-4 font-sans text-xs text-neutral-200 select-none">
      {/* Target Hardware Architecture Selector */}
      <div className="p-3.5 bg-neutral-900 border border-neutral-800 rounded-lg space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Monitor className="w-4 h-4 text-cyan-400" />
            <span className="font-semibold text-white text-xs">Simulated Target Hardware Architecture</span>
          </div>
          <span className="text-[11px] text-emerald-400 font-mono font-medium">{hardware.architecture}</span>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          {(Object.keys(HARDWARE_PROFILES) as ArchType[]).map((archKey) => {
            const prof = HARDWARE_PROFILES[archKey];
            const isSelected = hardware.id === archKey;
            return (
              <button
                key={archKey}
                onClick={() => onSelectHardware(archKey)}
                className={`p-2.5 rounded-lg border text-left transition-all flex flex-col gap-1 ${
                  isSelected
                    ? 'bg-neutral-850 border-amber-500 text-white shadow-xs'
                    : 'bg-neutral-950 border-neutral-800 hover:bg-neutral-800 text-neutral-300'
                }`}
              >
                <div className="flex items-center justify-between">
                  <span className="font-semibold text-xs text-white">{prof.name}</span>
                  <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-neutral-800 text-neutral-300">
                    {prof.bits}
                  </span>
                </div>
                <div className="text-[11px] text-neutral-400">{prof.tagline}</div>
                <div className="text-[10px] text-neutral-500 font-mono flex items-center gap-2 pt-0.5">
                  <span>RAM: {prof.ramTotalMB}MB</span>
                  <span>·</span>
                  <span>Backend: {prof.compositorBackend.split(' ')[0]}</span>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Compositor & Low-End Optimization Toggles */}
      <div className="p-3.5 bg-neutral-900 border border-neutral-800 rounded-lg space-y-3">
        <div className="flex items-center gap-2">
          <Zap className="w-4 h-4 text-amber-400" />
          <span className="font-semibold text-white text-xs">Compositor Performance & ARM Tuning</span>
        </div>

        <div className="space-y-3">
          {/* Low Spec Mode */}
          <div className="flex items-center justify-between p-2.5 bg-neutral-950 rounded-lg border border-neutral-800">
            <div>
              <div className="font-medium text-neutral-200">Ultra-Low Spec Hardware Mode</div>
              <div className="text-[11px] text-neutral-400">
                Disables backdrop blur shaders, switches to tiny-skia software rasterizer blit, direct KMS scanout.
              </div>
            </div>
            <button
              onClick={() => onUpdateConfig({ lowSpecMode: !config.lowSpecMode })}
              className={`px-3 py-1.5 rounded font-mono text-xs font-semibold transition-colors ${
                config.lowSpecMode
                  ? 'bg-emerald-500 text-neutral-950'
                  : 'bg-neutral-800 text-neutral-300 hover:bg-neutral-700'
              }`}
            >
              {config.lowSpecMode ? 'ENABLED' : 'DISABLED'}
            </button>
          </div>

          {/* Damage Tracking */}
          <div className="flex items-center justify-between p-2.5 bg-neutral-950 rounded-lg border border-neutral-800">
            <div>
              <div className="font-medium text-neutral-200">Smithay Damage Tracking (Re-render Dirty Rects Only)</div>
              <div className="text-[11px] text-neutral-400">
                Avoids re-rasterizing static desktop areas; reduces CPU utilization by ~70% on ARM SBCs.
              </div>
            </div>
            <button
              onClick={() => onUpdateConfig({ damageTracking: !config.damageTracking })}
              className={`px-3 py-1.5 rounded font-mono text-xs font-semibold transition-colors ${
                config.damageTracking
                  ? 'bg-emerald-500 text-neutral-950'
                  : 'bg-neutral-800 text-neutral-300 hover:bg-neutral-700'
              }`}
            >
              {config.damageTracking ? 'ENABLED' : 'DISABLED'}
            </button>
          </div>

          {/* Window Manager Layout Mode */}
          <div className="flex items-center justify-between p-2.5 bg-neutral-950 rounded-lg border border-neutral-800">
            <div>
              <div className="font-medium text-neutral-200">Window Management Mode</div>
              <div className="text-[11px] text-neutral-400">
                Switch between traditional floating windows and automatic dynamic tiling (BSP).
              </div>
            </div>
            <div className="flex items-center gap-1 bg-neutral-800 p-1 rounded-lg">
              <button
                onClick={() => onUpdateConfig({ tilingMode: false })}
                className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors ${
                  !config.tilingMode ? 'bg-neutral-700 text-white shadow-xs' : 'text-neutral-400 hover:text-white'
                }`}
              >
                Floating
              </button>
              <button
                onClick={() => onUpdateConfig({ tilingMode: true })}
                className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors ${
                  config.tilingMode ? 'bg-neutral-700 text-white shadow-xs' : 'text-neutral-400 hover:text-white'
                }`}
              >
                Auto-Tiling
              </button>
            </div>
          </div>
        </div>
      </div>

      <div className="p-3.5 bg-neutral-900 border border-neutral-800 rounded-lg space-y-3">
        <div className="flex items-center gap-2">
          <Layout className="w-4 h-4 text-sky-400" />
          <span className="font-semibold text-white text-xs">Dock and menu bar</span>
        </div>
        <div className="flex items-center justify-between gap-3 p-2.5 bg-neutral-950 rounded-lg border border-neutral-800">
          <div>
            <div className="font-medium text-neutral-200">Size</div>
            <div className="text-[11px] text-neutral-400">
              Scales the top menu bar and the dock together.
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-1 bg-neutral-800 p-1 rounded-lg" role="group" aria-label="Dock and menu bar size">
            {([
              ['small', 'Small'],
              ['default', 'Default'],
              ['large', 'Large'],
            ] as const).map(([size, label]) => (
              <button
                key={size}
                type="button"
                aria-pressed={config.chromeSize === size}
                onClick={() => onUpdateConfig({ chromeSize: size satisfies ChromeSize })}
                className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors ${
                  config.chromeSize === size ? 'bg-neutral-700 text-white shadow-xs' : 'text-neutral-400 hover:text-white'
                }`}
              >
                {label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Dynamic Theming Quick Presets */}
      <div className="p-3.5 bg-neutral-900 border border-neutral-800 rounded-lg space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Palette className="w-4 h-4 text-purple-400" />
            <span className="font-semibold text-white text-xs">Active Theme Preset</span>
          </div>
          <span className="text-[11px] text-amber-400 font-mono font-medium">{config.activeTheme.name}</span>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          {Object.keys(PRESET_THEMES).map((key) => {
            const th = PRESET_THEMES[key];
            const isSelected = config.activeTheme.id === th.id;
            return (
              <button
                key={th.id}
                onClick={() => handleSelectPreset(key)}
                className={`p-2.5 rounded-lg border text-left flex items-center justify-between transition-colors ${
                  isSelected
                    ? 'bg-neutral-800 border-amber-500 text-white shadow-xs'
                    : 'bg-neutral-950 border-neutral-800 text-neutral-300 hover:bg-neutral-850'
                }`}
              >
                <div>
                  <div className="font-semibold text-xs text-white">{th.name}</div>
                  <div className="text-[10px] text-neutral-400">{th.fontFamily}</div>
                </div>
                <div className="flex items-center gap-1">
                  <span className="w-3 h-3 rounded-full" style={{ backgroundColor: th.palette.surface }} />
                  <span className="w-3 h-3 rounded-full" style={{ backgroundColor: th.palette.accent }} />
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Wallpapers */}
      <div className="p-3.5 bg-neutral-900 border border-neutral-800 rounded-lg space-y-3">
        <div className="font-semibold text-white text-xs">Desktop Background</div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          {wallpapers.map((wp) => (
            <button
              key={wp.id}
              onClick={() => onUpdateConfig({ wallpaper: wp.id })}
              className={`p-2.5 rounded-lg border text-left transition-colors ${
                config.wallpaper === wp.id
                  ? 'bg-neutral-800 border-amber-500 text-white'
                  : 'bg-neutral-950 border-neutral-800 text-neutral-300 hover:bg-neutral-850'
              }`}
            >
              <div className="font-medium text-white">{wp.name}</div>
              <div className="text-[10px] text-neutral-400">{wp.desc}</div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
