'use client';

import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  AppDefinition,
  AppId,
  WindowState,
  HardwareProfile,
  HARDWARE_PROFILES,
  ArchType,
  DEConfig,
  PRESET_THEMES,
  DynamicTheme,
  OptimizationSettings,
} from '@/lib/oxide-types';
import TopPanel from './TopPanel';
import QuickSettings from './QuickSettings';
import AppLauncher from './AppLauncher';
import WindowFrame from './WindowFrame';

// Apps
import TerminalApp from '@/components/apps/TerminalApp';
import SystemMonitorApp from '@/components/apps/SystemMonitorApp';
import FileManagerApp from '@/components/apps/FileManagerApp';
import RustCodebaseApp from '@/components/apps/RustCodebaseApp';
import BenchmarkApp from '@/components/apps/BenchmarkApp';
import SettingsApp from '@/components/apps/SettingsApp';
import GuideApp from '@/components/apps/GuideApp';
import ThemeStudioApp from '@/components/apps/ThemeStudioApp';
import ProfilerApp from '@/components/apps/ProfilerApp';

import {
  Terminal,
  Activity,
  Folder,
  FileCode,
  Gauge,
  Settings,
  BookOpen,
  Palette,
  Cpu,
} from 'lucide-react';

function AppGlyph({ id, className = 'w-5 h-5' }: { id: AppId; className?: string }) {
  switch (id) {
    case 'theme-studio':
      return <Palette className={`${className} text-purple-400`} />;
    case 'profiler':
      return <Cpu className={`${className} text-cyan-400`} />;
    case 'terminal':
      return <Terminal className={`${className} text-amber-400`} />;
    case 'sysmon':
      return <Activity className={`${className} text-emerald-400`} />;
    case 'codebase':
      return <FileCode className={`${className} text-amber-300`} />;
    case 'benchmark':
      return <Gauge className={`${className} text-rose-400`} />;
    case 'files':
      return <Folder className={`${className} text-cyan-400`} />;
    case 'settings':
      return <Settings className={`${className} text-blue-400`} />;
    case 'guide':
      return <BookOpen className={`${className} text-amber-300`} />;
    default:
      return <Terminal className={`${className} text-neutral-400`} />;
  }
}

export const APPS: AppDefinition[] = [
  {
    id: 'terminal',
    title: 'Oxide Terminal',
    iconName: 'terminal',
    category: 'System',
    description: 'Fast, GPU-accelerated Alacritty-like shell running in pure Rust',
    defaultWidth: 640,
    defaultHeight: 420,
  },
  {
    id: 'theme-studio',
    title: 'Dynamic Theme Studio',
    iconName: 'palette',
    category: 'System',
    description: 'Customize palettes, icon themes, typography, and export RON/TOML',
    defaultWidth: 840,
    defaultHeight: 540,
  },
  {
    id: 'profiler',
    title: 'ARM Core Profiler',
    iconName: 'cpu',
    category: 'Development',
    description: 'Microsecond flamegraph, ARM NEON SIMD blitter, and zero-alloc arena',
    defaultWidth: 780,
    defaultHeight: 520,
  },
  {
    id: 'sysmon',
    title: 'System Monitor',
    iconName: 'activity',
    category: 'System',
    description: 'Real-time CPU cores, memory telemetry, and compositor render pipeline',
    defaultWidth: 700,
    defaultHeight: 460,
  },
  {
    id: 'codebase',
    title: 'Rust Architecture & Source Studio',
    iconName: 'file-code',
    category: 'Development',
    description: 'Browse complete Rust crates (Smithay, tiny-skia, DRM) and export ZIP',
    defaultWidth: 800,
    defaultHeight: 520,
  },
  {
    id: 'benchmark',
    title: 'Hardware Benchmark & Stress Test',
    iconName: 'gauge',
    category: 'Utilities',
    description: 'Stress test FPS, latency, and damage tracking on low-end ARM',
    defaultWidth: 680,
    defaultHeight: 450,
  },
  {
    id: 'files',
    title: 'Oxide Files',
    iconName: 'folder',
    category: 'Utilities',
    description: 'Lightweight dual-pane file explorer for Linux & ARM filesystem',
    defaultWidth: 650,
    defaultHeight: 430,
  },
  {
    id: 'settings',
    title: 'Oxide Settings',
    iconName: 'settings',
    category: 'System',
    description: 'Compositor performance, low-spec ARM tuning, themes, and display scale',
    defaultWidth: 620,
    defaultHeight: 460,
  },
  {
    id: 'guide',
    title: 'Hardware Deployment Guide',
    iconName: 'book-open',
    category: 'Documentation',
    description: 'Step-by-step setup for Raspberry Pi 2/3/4/5, Alpine, Arch ARM, and seatd',
    defaultWidth: 660,
    defaultHeight: 480,
  },
];

export default function DesktopEnvironment() {
  const [currentHardwareId, setCurrentHardwareId] = useState<ArchType>('armv7l');
  const currentWorkspace = 1;
  const [launcherOpen, setLauncherOpen] = useState(false);
  const [quickSettingsOpen, setQuickSettingsOpen] = useState(false);
  const [brightness, setBrightness] = useState(100);
  const [activeWindowId, setActiveWindowId] = useState<string | null>('win-theme-studio');

  const hardware = HARDWARE_PROFILES[currentHardwareId];

  // Default configuration with Dynamic Theme and Optimization settings
  const [config, setConfig] = useState<DEConfig>({
    activeTheme: PRESET_THEMES['oxide-dark'],
    wallpaper: 'stock',
    tilingMode: false,
    lowSpecMode: hardware.defaultLowSpecMode,
    fontScale: 100,
    damageTracking: true,
    showFpsOverlay: false,
    animationsEnabled: true,
    optimizations: {
      neonSimd: true,
      bumpAllocator: true,
      kmsHardwarePlanes: true,
      cacheAlignment64: true,
      directScanout: true,
      damageTracking: true,
      ltoAggressive: true,
    },
  });

  // Default initial windows (Theme Studio and Profiler open to immediately demonstrate requested capabilities)
  const [windows, setWindows] = useState<WindowState[]>([
    {
      id: 'win-theme-studio',
      appId: 'theme-studio',
      title: 'Oxide Dynamic Theme Engine',
      x: 28,
      y: 48,
      width: 780,
      height: 500,
      isMinimized: false,
      isMaximized: false,
      zIndex: 10,
      workspace: 1,
    },
    {
      id: 'win-profiler',
      appId: 'profiler',
      title: 'Oxide ARM Performance Profiler',
      x: 180,
      y: 90,
      width: 720,
      height: 470,
      isMinimized: false,
      isMaximized: false,
      zIndex: 11,
      workspace: 1,
    },
  ]);

  // Sync low-spec mode with hardware profile changes
  const handleSelectHardware = (arch: ArchType) => {
    setCurrentHardwareId(arch);
    const newProf = HARDWARE_PROFILES[arch];
    setConfig((c) => ({
      ...c,
      lowSpecMode: newProf.defaultLowSpecMode,
      activeTheme: newProf.bits === '32-bit' && c.lowSpecMode
        ? PRESET_THEMES['monochrome-lowres']
        : c.activeTheme,
    }));
  };

  const handleUpdateConfig = (partial: Partial<DEConfig>) => {
    setConfig((prev) => ({ ...prev, ...partial }));
  };

  const handleApplyTheme = (newTheme: DynamicTheme) => {
    setConfig((prev) => ({
      ...prev,
      activeTheme: newTheme,
    }));
  };

  const handleUpdateOptimizations = (partial: Partial<OptimizationSettings>) => {
    setConfig((prev) => ({
      ...prev,
      optimizations: {
        ...prev.optimizations,
        ...partial,
      },
    }));
  };

  const closeTimers = useRef<Map<string, number>>(new Map());

  const openApp = useCallback((appId: AppId) => {
    const appDef = APPS.find((a) => a.id === appId);
    if (!appDef) return;

    // Check if window already exists in current workspace
    const existing = windows.find(
      (w) => w.appId === appId && w.workspace === currentWorkspace && !w.isClosing
    );
    if (existing) {
      if (existing.isMinimized) {
        setWindows((prev) =>
          prev.map((w) => (w.id === existing.id ? { ...w, isMinimized: false } : w))
        );
      }
      setActiveWindowId(existing.id);
      return;
    }

    // Cascade position
    const offset = (windows.length % 5) * 28 + 40;
    const newId = `win-${appId}-${Date.now()}`;
    const maxZ = Math.max(0, ...windows.map((w) => w.zIndex)) + 1;

    const newWin: WindowState = {
      id: newId,
      appId,
      title: appDef.title,
      x: Math.min(offset + 20, 240),
      y: Math.min(offset + 30, 140),
      width: Math.min(appDef.defaultWidth, typeof window !== 'undefined' ? window.innerWidth - 60 : 720),
      height: appDef.defaultHeight,
      isMinimized: false,
      isMaximized: false,
      zIndex: maxZ,
      workspace: currentWorkspace,
    };

    setWindows((prev) => [...prev, newWin]);
    setActiveWindowId(newId);
  }, [currentWorkspace, windows]);

  const closeWindow = (id: string) => {
    if (closeTimers.current.has(id)) return;
    setWindows((prev) => prev.map((w) => (w.id === id ? { ...w, isClosing: true } : w)));
    if (activeWindowId === id) {
      const remaining = windows.filter((w) => w.id !== id && !w.isClosing);
      setActiveWindowId(remaining.length > 0 ? remaining[remaining.length - 1].id : null);
    }
    const timer = window.setTimeout(() => {
      closeTimers.current.delete(id);
      setWindows((prev) => prev.filter((w) => w.id !== id));
      setActiveWindowId((current) => (current === id ? null : current));
    }, 2000);
    closeTimers.current.set(id, timer);
  };

  useEffect(() => {
    const timers = closeTimers.current;
    return () => {
      timers.forEach((timer) => window.clearTimeout(timer));
    };
  }, []);

  const minimizeWindow = (id: string) => {
    setWindows((prev) => prev.map((w) => (w.id === id ? { ...w, isMinimized: true } : w)));
    if (activeWindowId === id) {
      setActiveWindowId(null);
    }
  };

  const toggleMaximizeWindow = (id: string) => {
    setWindows((prev) =>
      prev.map((w) => (w.id === id ? { ...w, isMaximized: !w.isMaximized } : w))
    );
  };

  const focusWindow = (id: string) => {
    setActiveWindowId(id);
    setWindows((prev) => {
      const maxZ = Math.max(0, ...prev.map((w) => w.zIndex)) + 1;
      return prev.map((w) => (w.id === id ? { ...w, zIndex: maxZ, isMinimized: false } : w));
    });
  };

  const updateWindowPosition = (id: string, x: number, y: number) => {
    setWindows((prev) => prev.map((w) => (w.id === id ? { ...w, x, y } : w)));
  };

  const updateWindowSize = (id: string, width: number, height: number) => {
    setWindows((prev) => prev.map((w) => (w.id === id ? { ...w, width, height } : w)));
  };

  // Global Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Super / Windows key or Alt+Space -> Launcher
      if ((e.key === 'Meta' && !e.ctrlKey) || (e.altKey && e.code === 'Space')) {
        e.preventDefault();
        setLauncherOpen((prev) => !prev);
      }
      // Super+Return -> Terminal
      if (e.metaKey && e.key === 'Enter') {
        e.preventDefault();
        openApp('terminal');
      }
      // Super+T -> Toggle Tiling
      if (e.metaKey && (e.key === 't' || e.key === 'T')) {
        e.preventDefault();
        setConfig((c) => ({ ...c, tilingMode: !c.tilingMode }));
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [openApp]);

  const currentWorkspaceWindows = windows.filter((w) => w.workspace === currentWorkspace);
  const theme = config.activeTheme;
  const screenBrightness = brightness === 100 ? undefined : { filter: `brightness(${brightness}%)` };

  // Background style renderer
  const getWallpaperClasses = () => {
    switch (config.wallpaper) {
      case 'oxide-mesh':
        return 'bg-gradient-to-br from-neutral-950 via-[#0d0f17] to-neutral-950';
      case 'nordic-aurora':
        return 'bg-gradient-to-br from-[#070b14] via-[#091a24] to-[#040810]';
      case 'rust-metal':
        return 'bg-gradient-to-br from-[#12100e] via-[#1a1410] to-[#0d0b0a]';
      case 'minimal-charcoal':
        return 'bg-[#0a0a0c]';
      default:
        return 'bg-neutral-950';
    }
  };

  const renderAppContent = (appId: AppId) => {
    switch (appId) {
      case 'theme-studio':
        return <ThemeStudioApp currentTheme={theme} onApplyTheme={handleApplyTheme} />;
      case 'profiler':
        return (
          <ProfilerApp
            hardware={hardware}
            config={config}
            onUpdateOptimizations={handleUpdateOptimizations}
          />
        );
      case 'terminal':
        return <TerminalApp hardware={hardware} config={config} onUpdateConfig={handleUpdateConfig} />;
      case 'sysmon':
        return (
          <SystemMonitorApp
            hardware={hardware}
            config={config}
            onToggleLowSpec={() => handleUpdateConfig({ lowSpecMode: !config.lowSpecMode })}
          />
        );
      case 'codebase':
        return <RustCodebaseApp />;
      case 'benchmark':
        return <BenchmarkApp hardware={hardware} config={config} />;
      case 'files':
        return <FileManagerApp />;
      case 'settings':
        return (
          <SettingsApp
            config={config}
            hardware={hardware}
            onUpdateConfig={handleUpdateConfig}
            onSelectHardware={handleSelectHardware}
          />
        );
      case 'guide':
        return <GuideApp />;
      default:
        return <div className="p-4 text-neutral-400">Application not found</div>;
    }
  };

  return (
    <div
      style={{
        backgroundColor: theme.palette.background,
        fontFamily: theme.fontFamily === 'System Sans' ? 'inherit' : theme.fontFamily,
      }}
      className="relative w-full h-screen overflow-hidden select-none flex flex-col font-sans transition-colors duration-200 bg-neutral-950"
    >
      {config.wallpaper === 'stock' ? (
        <div
          aria-hidden
          className="absolute inset-0 pointer-events-none bg-cover bg-center"
          style={{ backgroundImage: 'url(/wallpapers/stock.svg)', ...screenBrightness }}
        />
      ) : (
        <div
          aria-hidden
          className={`absolute inset-0 pointer-events-none ${getWallpaperClasses()}`}
          style={screenBrightness}
        />
      )}
      {config.wallpaper !== 'stock' && (
        <div className="absolute inset-0 pointer-events-none opacity-20" style={screenBrightness}>
          <svg className="w-full h-full" xmlns="http://www.w3.org/2000/svg">
            <defs>
              <pattern id="grid-pattern" width="48" height="48" patternUnits="userSpaceOnUse">
                <path d="M 48 0 L 0 0 0 48" fill="none" stroke="rgba(255,255,255,0.04)" strokeWidth="1" />
              </pattern>
            </defs>
            <rect width="100%" height="100%" fill="url(#grid-pattern)" />
          </svg>
        </div>
      )}

      {/* Top Status Bar (Wayland Layer-shell) */}
      <TopPanel
        config={config}
        onToggleLauncher={() => setLauncherOpen((o) => !o)}
        onToggleQuickSettings={() => setQuickSettingsOpen((o) => !o)}
        quickSettingsOpen={quickSettingsOpen}
        onOpenApp={openApp}
      />

      {/* Quick Settings Flyout */}
      <QuickSettings
        isOpen={quickSettingsOpen}
        onClose={() => setQuickSettingsOpen(false)}
        hardware={hardware}
        config={config}
        onUpdateConfig={handleUpdateConfig}
        onSelectHardware={handleSelectHardware}
        onOpenApp={openApp}
        brightness={brightness}
        onBrightnessChange={setBrightness}
      />

      {/* Application Launcher Modal */}
      <AppLauncher
        isOpen={launcherOpen}
        onClose={() => setLauncherOpen(false)}
        apps={APPS}
        onLaunchApp={openApp}
      />

      {/* Desktop Workspace Canvas */}
      <main
        className="relative flex-1 overflow-hidden"
        style={screenBrightness}
        onClick={() => setQuickSettingsOpen(false)}
      >
        {/* Dynamic Tiling Container OR Free-floating Window Manager Canvas */}
        {config.tilingMode ? (
          <div className="absolute inset-0 pt-2 p-3 grid grid-cols-1 md:grid-cols-2 gap-3 h-full">
            {currentWorkspaceWindows
              .filter((w) => !w.isMinimized)
              .map((win) => (
                <div key={win.id} className="h-full min-h-[300px]">
                  <WindowFrame
                    window={win}
                    isActive={activeWindowId === win.id}
                    config={config}
                    onFocus={() => focusWindow(win.id)}
                    onClose={() => closeWindow(win.id)}
                    onMinimize={() => minimizeWindow(win.id)}
                    onToggleMaximize={() => toggleMaximizeWindow(win.id)}
                    onUpdatePosition={(x, y) => updateWindowPosition(win.id, x, y)}
                    onUpdateSize={(w, h) => updateWindowSize(win.id, w, h)}
                  >
                    {renderAppContent(win.appId)}
                  </WindowFrame>
                </div>
              ))}
          </div>
        ) : (
          <div className="relative w-full h-full">
            {currentWorkspaceWindows.map((win) => (
              <WindowFrame
                key={win.id}
                window={win}
                isActive={activeWindowId === win.id}
                config={config}
                onFocus={() => focusWindow(win.id)}
                onClose={() => closeWindow(win.id)}
                onMinimize={() => minimizeWindow(win.id)}
                onToggleMaximize={() => toggleMaximizeWindow(win.id)}
                onUpdatePosition={(x, y) => updateWindowPosition(win.id, x, y)}
                onUpdateSize={(w, h) => updateWindowSize(win.id, w, h)}
              >
                {renderAppContent(win.appId)}
              </WindowFrame>
            ))}
          </div>
        )}
      </main>

      <nav
        aria-label="Dock"
        style={{
          backgroundColor: 'transparent',
          borderColor: theme.palette.textSecondary,
          ...screenBrightness,
        }}
        className={`absolute bottom-3 left-1/2 z-40 flex -translate-x-1/2 items-end gap-1 border-4 px-2 py-1.5 select-none ${
          theme.windowStyle.isPillShape ? 'rounded-full' : 'rounded-[22px]'
        }`}
      >
        {APPS.map((app) => {
          const appWindows = windows.filter((win) => win.appId === app.id && !win.isClosing);
          const focused = appWindows.some((win) => win.id === activeWindowId && !win.isMinimized);
          return (
            <button
              key={app.id}
              type="button"
              title={app.title}
              aria-label={app.title}
              onClick={() => {
                if (appWindows.length === 0) {
                  openApp(app.id);
                  return;
                }
                const current = appWindows.find((win) => win.id === activeWindowId && !win.isMinimized);
                if (current) {
                  minimizeWindow(current.id);
                  return;
                }
                const next = appWindows.find((win) => win.isMinimized) ?? appWindows[appWindows.length - 1];
                focusWindow(next.id);
              }}
              className="group flex w-12 flex-col items-center gap-1"
            >
              <span
                style={{
                  backgroundColor: theme.palette.surface,
                  boxShadow: focused
                    ? `inset 0 0 0 2px ${theme.palette.accent}`
                    : `inset 0 0 0 1px ${theme.palette.border}`,
                }}
                className={`flex h-10 w-10 items-center justify-center transition-transform group-hover:-translate-y-1 group-hover:scale-110 ${
                  theme.windowStyle.isPillShape ? 'rounded-full' : 'rounded-xl'
                }`}
              >
                <AppGlyph id={app.id} />
              </span>
              <span
                className="h-1 w-1 rounded-full"
                style={{ backgroundColor: appWindows.length > 0 ? theme.palette.textPrimary : 'transparent' }}
              />
            </button>
          );
        })}
      </nav>
    </div>
  );
}
