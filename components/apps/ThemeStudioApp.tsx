'use client';

import React, { useState } from 'react';
import {
  DynamicTheme,
  PRESET_THEMES,
  ColorPalette,
  FontFamilyType,
  IconThemeType,
  WindowStyleConfig,
} from '@/lib/oxide-types';
import {
  Palette,
  Type,
  Layout,
  Download,
  Copy,
  Check,
  Sparkles,
  Maximize2,
  Minus,
  X,
  Sliders,
  RefreshCw,
} from 'lucide-react';

interface ThemeStudioAppProps {
  currentTheme: DynamicTheme;
  onApplyTheme: (theme: DynamicTheme) => void;
}

export default function ThemeStudioApp({ currentTheme, onApplyTheme }: ThemeStudioAppProps) {
  const [activeTab, setActiveTab] = useState<'presets' | 'palette' | 'typography' | 'window' | 'export'>('presets');
  const [editingTheme, setEditingTheme] = useState<DynamicTheme>(currentTheme);
  const [copiedType, setCopiedType] = useState<string | null>(null);

  const handlePresetSelect = (presetKey: string) => {
    const selected = PRESET_THEMES[presetKey];
    if (selected) {
      const cloned = JSON.parse(JSON.stringify(selected));
      setEditingTheme(cloned);
      onApplyTheme(cloned);
    }
  };

  const handlePaletteChange = (key: keyof ColorPalette, value: string) => {
    const updated: DynamicTheme = {
      ...editingTheme,
      name: editingTheme.name.includes('(Custom)') ? editingTheme.name : `${editingTheme.name} (Custom)`,
      palette: {
        ...editingTheme.palette,
        [key]: value,
      },
    };
    setEditingTheme(updated);
    onApplyTheme(updated);
  };

  const handleFontChange = (fontFamily: FontFamilyType) => {
    const updated: DynamicTheme = {
      ...editingTheme,
      fontFamily,
    };
    setEditingTheme(updated);
    onApplyTheme(updated);
  };

  const handleIconThemeChange = (iconTheme: IconThemeType) => {
    const updated: DynamicTheme = {
      ...editingTheme,
      iconTheme,
    };
    setEditingTheme(updated);
    onApplyTheme(updated);
  };

  const handleWindowStyleChange = (partial: Partial<WindowStyleConfig>) => {
    const updated: DynamicTheme = {
      ...editingTheme,
      windowStyle: {
        ...editingTheme.windowStyle,
        ...partial,
      },
    };
    setEditingTheme(updated);
    onApplyTheme(updated);
  };

  const handleCopyCode = (text: string, type: string) => {
    navigator.clipboard.writeText(text);
    setCopiedType(type);
    setTimeout(() => setCopiedType(null), 2000);
  };

  // Generate RON export format
  const generateRon = () => {
    return `(\n  name: "${editingTheme.name}",\n  font_family: "${editingTheme.fontFamily}",\n  icon_theme: "${editingTheme.iconTheme}",\n  palette: (\n    background: "${editingTheme.palette.background}",\n    surface: "${editingTheme.palette.surface}",\n    panel: "${editingTheme.palette.panel}",\n    border: "${editingTheme.palette.border}",\n    text_primary: "${editingTheme.palette.textPrimary}",\n    text_secondary: "${editingTheme.palette.textSecondary}",\n    accent: "${editingTheme.palette.accent}",\n    accent_hover: "${editingTheme.palette.accentHover}",\n    success: "${editingTheme.palette.success}",\n    warning: "${editingTheme.palette.warning}",\n    danger: "${editingTheme.palette.danger}",\n  ),\n  window_style: (\n    border_radius: ${editingTheme.windowStyle.borderRadius},\n    border_width: ${editingTheme.windowStyle.borderWidth},\n    backdrop_blur: ${editingTheme.windowStyle.backdropBlur},\n    window_controls: "${editingTheme.windowStyle.windowControls}",\n    compact_titlebar: ${editingTheme.windowStyle.compactTitlebar},\n  ),\n)`;
  };

  // Generate TOML export format
  const generateToml = () => {
    return `[theme]\nname = "${editingTheme.name}"\nfont_family = "${editingTheme.fontFamily}"\nicon_theme = "${editingTheme.iconTheme}"\n\n[palette]\nbackground = "${editingTheme.palette.background}"\nsurface = "${editingTheme.palette.surface}"\npanel = "${editingTheme.palette.panel}"\nborder = "${editingTheme.palette.border}"\ntext_primary = "${editingTheme.palette.textPrimary}"\naccent = "${editingTheme.palette.accent}"\nsuccess = "${editingTheme.palette.success}"\nwarning = "${editingTheme.palette.warning}"\ndanger = "${editingTheme.palette.danger}"\n\n[window_style]\nborder_radius = ${editingTheme.windowStyle.borderRadius}\nborder_width = ${editingTheme.windowStyle.borderWidth}\nbackdrop_blur = ${editingTheme.windowStyle.backdropBlur}\nwindow_controls = "${editingTheme.windowStyle.windowControls}"\ncompact_titlebar = ${editingTheme.windowStyle.compactTitlebar}`;
  };

  return (
    <div className="flex flex-col h-full bg-neutral-950 text-neutral-200 font-sans select-none text-xs overflow-hidden">
      {/* Top Header */}
      <div className="flex flex-wrap items-center justify-between gap-3 px-4 py-2.5 bg-neutral-900 border-b border-neutral-800 shrink-0">
        <div className="flex items-center gap-2">
          <Palette className="w-4 h-4 text-amber-400" />
          <span className="font-semibold text-white text-sm">Oxide Dynamic Theme Engine</span>
          <span className="text-[11px] text-neutral-400">·</span>
          <span className="text-amber-400 font-medium font-mono text-[11px]">{editingTheme.name}</span>
        </div>

        {/* Tab switcher */}
        <div className="flex items-center gap-1 bg-neutral-950 p-1 rounded-lg border border-neutral-800">
          <button
            onClick={() => setActiveTab('presets')}
            className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors ${
              activeTab === 'presets' ? 'bg-neutral-800 text-white' : 'text-neutral-400 hover:text-white'
            }`}
          >
            Presets
          </button>
          <button
            onClick={() => setActiveTab('palette')}
            className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors ${
              activeTab === 'palette' ? 'bg-neutral-800 text-white' : 'text-neutral-400 hover:text-white'
            }`}
          >
            Color Palette
          </button>
          <button
            onClick={() => setActiveTab('typography')}
            className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors ${
              activeTab === 'typography' ? 'bg-neutral-800 text-white' : 'text-neutral-400 hover:text-white'
            }`}
          >
            Fonts & Icons
          </button>
          <button
            onClick={() => setActiveTab('window')}
            className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors ${
              activeTab === 'window' ? 'bg-neutral-800 text-white' : 'text-neutral-400 hover:text-white'
            }`}
          >
            Window Styling
          </button>
          <button
            onClick={() => setActiveTab('export')}
            className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors ${
              activeTab === 'export' ? 'bg-neutral-800 text-white' : 'text-neutral-400 hover:text-white'
            }`}
          >
            Export (RON / TOML)
          </button>
        </div>
      </div>

      {/* Main Split: Control Panel on Left + Live Theme Preview on Right */}
      <div className="flex-1 flex flex-col md:flex-row overflow-hidden">
        {/* Left Side: Controls for Selected Tab */}
        <div className="w-full md:w-80 lg:w-96 p-4 overflow-y-auto border-r border-neutral-800 space-y-4 shrink-0 bg-neutral-900/40">
          {activeTab === 'presets' && (
            <div className="space-y-3">
              <div className="font-semibold text-white text-xs">Curated Desktop Presets</div>
              <p className="text-[11px] text-neutral-400">
                Instantly switch between hand-crafted color schemes tested for high-contrast legibility and low-resource ARM efficiency.
              </p>

              <div className="space-y-2 pt-1">
                {Object.keys(PRESET_THEMES).map((key) => {
                  const th = PRESET_THEMES[key];
                  const isSelected = editingTheme.id === th.id;
                  return (
                    <button
                      key={th.id}
                      onClick={() => handlePresetSelect(key)}
                      className={`w-full text-left p-3 rounded-lg border transition-all flex flex-col gap-1.5 ${
                        isSelected
                          ? 'bg-neutral-850 border-amber-500 shadow-xs'
                          : 'bg-neutral-950 border-neutral-800 hover:bg-neutral-850 hover:border-neutral-700'
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-semibold text-xs text-white">{th.name}</span>
                        {/* 4-color swatch */}
                        <div className="flex items-center gap-1">
                          <span className="w-3 h-3 rounded-full" style={{ backgroundColor: th.palette.background }} />
                          <span className="w-3 h-3 rounded-full" style={{ backgroundColor: th.palette.surface }} />
                          <span className="w-3 h-3 rounded-full" style={{ backgroundColor: th.palette.accent }} />
                          <span className="w-3 h-3 rounded-full" style={{ backgroundColor: th.palette.border }} />
                        </div>
                      </div>
                      <p className="text-[11px] text-neutral-400">{th.description}</p>
                      <div className="flex items-center gap-2 text-[10px] text-neutral-500 font-mono pt-1">
                        <span>Font: {th.fontFamily}</span>
                        <span>·</span>
                        <span>Radius: {th.windowStyle.borderRadius}px</span>
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {activeTab === 'palette' && (
            <div className="space-y-3">
              <div className="font-semibold text-white text-xs">Customizable Color Palette</div>
              <p className="text-[11px] text-neutral-400">
                Adjust individual color tokens. Changes immediately propagate to the compositor panel and window frames.
              </p>

              <div className="space-y-2 pt-1">
                {[
                  { key: 'background', label: 'Canvas Background', desc: 'Main root workspace desktop' },
                  { key: 'surface', label: 'Window Surface', desc: 'Window frame background' },
                  { key: 'panel', label: 'Status Bar Panel', desc: 'Top layer-shell panel' },
                  { key: 'border', label: 'Border Color', desc: 'Active & inactive hairline borders' },
                  { key: 'accent', label: 'Primary Accent', desc: 'Buttons, active indicators, highlights' },
                  { key: 'accentHover', label: 'Accent Hover', desc: 'Hover interactive states' },
                  { key: 'textPrimary', label: 'Primary Text', desc: 'Headlines, titles, foreground' },
                  { key: 'textSecondary', label: 'Secondary Text', desc: 'Muted captions, metadata' },
                  { key: 'success', label: 'Success Color', desc: 'Process nominal indicators' },
                  { key: 'warning', label: 'Warning Color', desc: 'Throttling and memory warnings' },
                  { key: 'danger', label: 'Alert / Danger', desc: 'Kill process, errors' },
                ].map((item) => {
                  const colorKey = item.key as keyof ColorPalette;
                  const val = editingTheme.palette[colorKey];
                  return (
                    <div
                      key={item.key}
                      className="p-2 bg-neutral-950 rounded-lg border border-neutral-800 flex items-center justify-between gap-3"
                    >
                      <div className="min-w-0 flex-1">
                        <div className="font-medium text-xs text-neutral-200">{item.label}</div>
                        <div className="text-[10px] text-neutral-500 truncate">{item.desc}</div>
                      </div>

                      <div className="flex items-center gap-2 shrink-0">
                        <input
                          type="color"
                          value={val}
                          onChange={(e) => handlePaletteChange(colorKey, e.target.value)}
                          className="w-7 h-7 rounded border border-neutral-700 bg-transparent cursor-pointer p-0"
                        />
                        <input
                          type="text"
                          value={val}
                          onChange={(e) => handlePaletteChange(colorKey, e.target.value)}
                          className="w-20 px-1.5 py-1 bg-neutral-900 rounded border border-neutral-700 text-neutral-300 font-mono text-[11px] uppercase text-center"
                        />
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {activeTab === 'typography' && (
            <div className="space-y-4">
              <div className="font-semibold text-white text-xs">Font Selection & Sizing</div>
              <p className="text-[11px] text-neutral-400">
                Choose typography tuned for high-density tabular numbers and code readability.
              </p>

              <div className="space-y-2">
                <label className="text-[11px] text-neutral-300 font-medium">Font Family</label>
                {(['Poppins', 'JetBrains Mono', 'Plus Jakarta Sans', 'DM Sans', 'Fira Code', 'System Sans'] as FontFamilyType[]).map(
                  (font) => (
                    <button
                      key={font}
                      onClick={() => handleFontChange(font)}
                      className={`w-full text-left p-2.5 rounded-lg border flex items-center justify-between transition-colors ${
                        editingTheme.fontFamily === font
                          ? 'bg-neutral-800 border-amber-500 text-white font-semibold'
                          : 'bg-neutral-950 border-neutral-800 text-neutral-300 hover:bg-neutral-850'
                      }`}
                    >
                      <span>{font}</span>
                      <span className="text-[10px] text-neutral-500">The quick brown fox 123</span>
                    </button>
                  )
                )}
              </div>

              <div className="space-y-2 pt-2 border-t border-neutral-800">
                <label className="text-[11px] text-neutral-300 font-medium">Icon Theme Set</label>
                {(['oxide-minimal', 'neo-glyph', 'lucid-wire', 'monochrome-lowres'] as IconThemeType[]).map((iconSet) => (
                  <button
                    key={iconSet}
                    onClick={() => handleIconThemeChange(iconSet)}
                    className={`w-full text-left p-2.5 rounded-lg border flex items-center justify-between transition-colors ${
                      editingTheme.iconTheme === iconSet
                        ? 'bg-neutral-800 border-amber-500 text-white font-semibold'
                        : 'bg-neutral-950 border-neutral-800 text-neutral-300 hover:bg-neutral-850'
                    }`}
                  >
                    <span className="capitalize">{iconSet.replace('-', ' ')}</span>
                    <span className="text-[10px] text-neutral-500 font-mono">SVG vector</span>
                  </button>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'window' && (
            <div className="space-y-4">
              <div className="font-semibold text-white text-xs">Window Styling & Compositor Geometry</div>
              <p className="text-[11px] text-neutral-400">
                Tailor window corner curvature, border thickness, and blur passes for modern aesthetics or low-resource direct scanout.
              </p>

              {/* Border Radius */}
              <div className="space-y-1.5 p-2.5 bg-neutral-950 rounded-lg border border-neutral-800">
                <div className="flex items-center justify-between">
                  <span className="font-medium text-neutral-200">Border Radius</span>
                  <span className="font-mono text-amber-400">{editingTheme.windowStyle.borderRadius}px</span>
                </div>
                <div className="flex items-center gap-1.5">
                  {[0, 4, 8, 12, 16, 24].map((rad) => (
                    <button
                      key={rad}
                      onClick={() => handleWindowStyleChange({ borderRadius: rad as WindowStyleConfig['borderRadius'] })}
                      className={`flex-1 py-1 text-center font-mono rounded text-[11px] transition-colors ${
                        editingTheme.windowStyle.borderRadius === rad
                          ? 'bg-amber-500 text-neutral-950 font-bold'
                          : 'bg-neutral-900 text-neutral-400 hover:bg-neutral-800'
                      }`}
                    >
                      {rad}px
                    </button>
                  ))}
                </div>
                <div className="text-[10px] text-neutral-500 pt-1">
                  0px enables hardware direct scanout bypass on 32-bit ARM (armv7l).
                </div>
              </div>

              {/* Backdrop Blur Toggle */}
              <div className="p-2.5 bg-neutral-950 rounded-lg border border-neutral-800 flex items-center justify-between">
                <div>
                  <div className="font-medium text-neutral-200">Acrylic Backdrop Blur</div>
                  <div className="text-[10px] text-neutral-500">Requires GLES / GPU shader pass</div>
                </div>
                <button
                  onClick={() => handleWindowStyleChange({ backdropBlur: !editingTheme.windowStyle.backdropBlur })}
                  className={`px-3 py-1 rounded font-mono text-[11px] font-bold transition-colors ${
                    editingTheme.windowStyle.backdropBlur
                      ? 'bg-emerald-500 text-neutral-950'
                      : 'bg-neutral-800 text-neutral-400'
                  }`}
                >
                  {editingTheme.windowStyle.backdropBlur ? 'ENABLED' : 'DISABLED'}
                </button>
              </div>

              {/* Window Controls Position */}
              <div className="p-2.5 bg-neutral-950 rounded-lg border border-neutral-800 flex items-center justify-between">
                <div>
                  <div className="font-medium text-neutral-200">Window Controls Position</div>
                  <div className="text-[10px] text-neutral-500">Minimize / Maximize / Close placement</div>
                </div>
                <div className="flex items-center gap-1 bg-neutral-900 p-0.5 rounded">
                  <button
                    onClick={() => handleWindowStyleChange({ windowControls: 'left' })}
                    className={`px-2 py-1 rounded text-[10px] ${
                      editingTheme.windowStyle.windowControls === 'left' ? 'bg-neutral-800 text-white' : 'text-neutral-400'
                    }`}
                  >
                    Left
                  </button>
                  <button
                    onClick={() => handleWindowStyleChange({ windowControls: 'right' })}
                    className={`px-2 py-1 rounded text-[10px] ${
                      editingTheme.windowStyle.windowControls === 'right' ? 'bg-neutral-800 text-white' : 'text-neutral-400'
                    }`}
                  >
                    Right
                  </button>
                </div>
              </div>

              {/* Compact Titlebar */}
              <div className="p-2.5 bg-neutral-950 rounded-lg border border-neutral-800 flex items-center justify-between">
                <div>
                  <div className="font-medium text-neutral-200">Compact Titlebar</div>
                  <div className="text-[10px] text-neutral-500">Saves vertical pixels on 7-inch SBC displays</div>
                </div>
                <button
                  onClick={() => handleWindowStyleChange({ compactTitlebar: !editingTheme.windowStyle.compactTitlebar })}
                  className={`px-3 py-1 rounded font-mono text-[11px] font-bold transition-colors ${
                    editingTheme.windowStyle.compactTitlebar
                      ? 'bg-emerald-500 text-neutral-950'
                      : 'bg-neutral-800 text-neutral-400'
                  }`}
                >
                  {editingTheme.windowStyle.compactTitlebar ? 'ON' : 'OFF'}
                </button>
              </div>
            </div>
          )}

          {activeTab === 'export' && (
            <div className="space-y-4">
              <div className="font-semibold text-white text-xs">Export Theme for Rust Compositor</div>
              <p className="text-[11px] text-neutral-400">
                Copy or save your customized theme configuration to <code className="text-amber-400">~/.config/oxide/theme.ron</code> or <code className="text-amber-400">theme.toml</code>.
              </p>

              {/* RON Export Box */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between text-[11px]">
                  <span className="font-mono text-neutral-300 font-semibold">theme.ron (Rust Object Notation)</span>
                  <button
                    onClick={() => handleCopyCode(generateRon(), 'ron')}
                    className="flex items-center gap-1 text-neutral-400 hover:text-white"
                  >
                    {copiedType === 'ron' ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                    <span>{copiedType === 'ron' ? 'Copied' : 'Copy RON'}</span>
                  </button>
                </div>
                <pre className="p-2.5 bg-neutral-950 rounded border border-neutral-800 font-mono text-[10px] text-neutral-300 overflow-x-auto max-h-48 whitespace-pre">
                  {generateRon()}
                </pre>
              </div>

              {/* TOML Export Box */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between text-[11px]">
                  <span className="font-mono text-neutral-300 font-semibold">theme.toml (Standard Config)</span>
                  <button
                    onClick={() => handleCopyCode(generateToml(), 'toml')}
                    className="flex items-center gap-1 text-neutral-400 hover:text-white"
                  >
                    {copiedType === 'toml' ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                    <span>{copiedType === 'toml' ? 'Copied' : 'Copy TOML'}</span>
                  </button>
                </div>
                <pre className="p-2.5 bg-neutral-950 rounded border border-neutral-800 font-mono text-[10px] text-neutral-300 overflow-x-auto max-h-36 whitespace-pre">
                  {generateToml()}
                </pre>
              </div>
            </div>
          )}
        </div>

        {/* Right Side: Interactive Live Theme Preview Sandbox */}
        <div className="flex-1 p-5 overflow-y-auto flex flex-col justify-center items-center bg-neutral-950">
          <div className="w-full max-w-lg space-y-3">
            <div className="flex items-center justify-between text-neutral-400 text-[11px]">
              <span>Real-Time Component Preview</span>
              <span className="font-mono text-emerald-400">Live Active Style</span>
            </div>

            {/* Simulated Desktop Window Frame with Active Theme */}
            <div
              style={{
                backgroundColor: editingTheme.palette.surface,
                borderColor: editingTheme.palette.border,
                borderRadius: `${editingTheme.windowStyle.borderRadius}px`,
                borderWidth: `${editingTheme.windowStyle.borderWidth}px`,
              }}
              className="border shadow-2xl overflow-hidden transition-all duration-200"
            >
              {/* Sample Window Titlebar */}
              <div
                style={{
                  backgroundColor: editingTheme.palette.panel,
                  borderColor: editingTheme.palette.border,
                  height: editingTheme.windowStyle.compactTitlebar ? '28px' : '34px',
                }}
                className={`px-3 flex items-center justify-between border-b relative`}
              >
                {editingTheme.windowStyle.isMacStyle || editingTheme.windowStyle.windowControls === 'left' ? (
                  <div className="flex items-center gap-1.5 z-10">
                    <span className="w-2.5 h-2.5 rounded-full bg-[#ff5f56] border border-[#e0443e]" />
                    <span className="w-2.5 h-2.5 rounded-full bg-[#ffbd2e] border border-[#dea123]" />
                    <span className="w-2.5 h-2.5 rounded-full bg-[#27c93f] border border-[#1aab29]" />
                  </div>
                ) : (
                  <div className="flex items-center gap-2 z-10">
                    <span
                      className={`w-2.5 h-2.5 ${editingTheme.windowStyle.isPillShape ? 'rounded-full' : 'rounded-xs'}`}
                      style={{ backgroundColor: editingTheme.palette.accent }}
                    />
                    <span
                      style={{ color: editingTheme.palette.textPrimary }}
                      className="font-semibold text-xs truncate"
                    >
                      Oxide Sample Window
                    </span>
                  </div>
                )}

                {(editingTheme.windowStyle.isMacStyle || editingTheme.windowStyle.windowControls === 'left') && (
                  <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
                    <span
                      style={{ color: editingTheme.palette.textPrimary }}
                      className="font-semibold text-xs truncate select-none"
                    >
                      Oxide Sample Window
                    </span>
                  </div>
                )}

                {!(editingTheme.windowStyle.isMacStyle || editingTheme.windowStyle.windowControls === 'left') && (
                  <div className={`flex items-center gap-1.5 z-10 ${editingTheme.windowStyle.isPillShape ? 'bg-neutral-900/60 px-2 py-0.5 rounded-full' : ''}`}>
                    <span className="w-2.5 h-2.5 rounded-full bg-neutral-600 opacity-60" />
                    <span className="w-2.5 h-2.5 rounded-full bg-neutral-600 opacity-60" />
                    <span className="w-2.5 h-2.5 rounded-full bg-rose-500/80" />
                  </div>
                )}
              </div>

              {/* Sample Window Body */}
              <div className="p-4 space-y-3.5" style={{ color: editingTheme.palette.textPrimary }}>
                <div className="flex items-center justify-between">
                  <div>
                    <h4 className="font-bold text-sm" style={{ color: editingTheme.palette.textPrimary }}>
                      {editingTheme.name}
                    </h4>
                    <p className="text-[11px]" style={{ color: editingTheme.palette.textSecondary }}>
                      Font: {editingTheme.fontFamily} · Radius: {editingTheme.windowStyle.borderRadius}px
                    </p>
                  </div>
                  <button
                    style={{
                      backgroundColor: editingTheme.palette.accent,
                      color: editingTheme.palette.background,
                      borderRadius: editingTheme.windowStyle.isPillShape ? '9999px' : `${Math.max(2, editingTheme.windowStyle.borderRadius - 4)}px`,
                    }}
                    className={`px-3 py-1 font-semibold text-xs shadow-xs transition-opacity hover:opacity-90 ${
                      editingTheme.windowStyle.isPillShape ? 'rounded-full px-4' : ''
                    }`}
                  >
                    Action Button
                  </button>
                </div>

                {/* Sample Input & Controls */}
                <div className="grid grid-cols-2 gap-2">
                  <div
                    style={{
                      backgroundColor: editingTheme.palette.background,
                      borderColor: editingTheme.palette.border,
                      borderRadius: `${Math.max(2, editingTheme.windowStyle.borderRadius - 4)}px`,
                    }}
                    className="p-2 border"
                  >
                    <div className="text-[10px]" style={{ color: editingTheme.palette.textSecondary }}>
                      Status Telemetry
                    </div>
                    <div className="font-mono font-bold text-xs" style={{ color: editingTheme.palette.success }}>
                      ● Active (0.78ms)
                    </div>
                  </div>

                  <div
                    style={{
                      backgroundColor: editingTheme.palette.background,
                      borderColor: editingTheme.palette.border,
                      borderRadius: `${Math.max(2, editingTheme.windowStyle.borderRadius - 4)}px`,
                    }}
                    className="p-2 border"
                  >
                    <div className="text-[10px]" style={{ color: editingTheme.palette.textSecondary }}>
                      Memory Overhead
                    </div>
                    <div className="font-mono font-bold text-xs" style={{ color: editingTheme.palette.accent }}>
                      28.4 MiB RSS
                    </div>
                  </div>
                </div>

                {/* Sample Tab Bar */}
                <div
                  style={{
                    backgroundColor: editingTheme.palette.background,
                    borderColor: editingTheme.palette.border,
                    borderRadius: `${Math.max(2, editingTheme.windowStyle.borderRadius - 4)}px`,
                  }}
                  className="p-1 border flex items-center gap-1"
                >
                  <span
                    style={{
                      backgroundColor: editingTheme.palette.surface,
                      color: editingTheme.palette.accent,
                    }}
                    className="px-2.5 py-1 rounded text-[11px] font-semibold"
                  >
                    Compositor
                  </span>
                  <span
                    style={{ color: editingTheme.palette.textSecondary }}
                    className="px-2.5 py-1 text-[11px]"
                  >
                    Wayland Shell
                  </span>
                  <span
                    style={{ color: editingTheme.palette.textSecondary }}
                    className="px-2.5 py-1 text-[11px]"
                  >
                    ARM SIMD
                  </span>
                </div>
              </div>
            </div>

            <div className="text-center pt-2">
              <button
                onClick={() => onApplyTheme(editingTheme)}
                style={{
                  backgroundColor: editingTheme.palette.accent,
                  color: editingTheme.palette.background,
                }}
                className="px-4 py-2 font-bold text-xs rounded-lg transition-transform hover:scale-105 shadow-md"
              >
                Apply Theme to Entire Desktop Now
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
