'use client';

import React, { useState, useEffect } from 'react';
import { DEConfig, AppDefinition, AppId } from '@/lib/oxide-types';
import { Terminal, Cpu, HardDrive, Wifi, Volume2, ToggleLeft, ToggleRight, LayoutGrid, Monitor, ShieldCheck } from 'lucide-react';

interface TopPanelProps {
  config: DEConfig;
  activeWindowTitle: string | null;
  onToggleLauncher: () => void;
  onToggleQuickSettings: () => void;
  quickSettingsOpen: boolean;
  onOpenApp: (appId: AppId) => void;
}

export default function TopPanel({
  config,
  activeWindowTitle,
  onToggleLauncher,
  onToggleQuickSettings,
  quickSettingsOpen,
  onOpenApp,
}: TopPanelProps) {
  const [timeStr, setTimeStr] = useState<string>('12:00');
  const [dateStr, setDateStr] = useState<string>('Oct 13');

  useEffect(() => {
    const updateTime = () => {
      const d = new Date();
      setTimeStr(d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }));
      setDateStr(d.toLocaleDateString([], { month: 'short', day: 'numeric' }));
    };
    updateTime();
    const interval = setInterval(updateTime, 10000);
    return () => clearInterval(interval);
  }, []);

  const theme = config.activeTheme;
  const isPill = theme.windowStyle.isPillShape;
  const isMac = theme.windowStyle.isMacStyle;

  return (
    <header
      style={{
        backgroundColor: theme.palette.panel,
        borderColor: theme.palette.border,
        color: theme.palette.textSecondary,
        fontFamily: theme.fontFamily === 'System Sans' ? 'inherit' : theme.fontFamily,
      }}
      className="h-9 w-full backdrop-blur-md border-b px-2 sm:px-3 flex items-center justify-between text-xs select-none z-50 shrink-0 transition-colors duration-200"
    >
      {/* Left Zone: Launcher Button */}
      <div className="flex items-center gap-2">
        {/* Oxide Brand / Launcher Button */}
        <button
          onClick={onToggleLauncher}
          style={{
            backgroundColor: theme.palette.accent,
            color: theme.palette.background,
          }}
          className={`flex items-center gap-1.5 px-2.5 py-1 font-bold text-xs transition-opacity hover:opacity-90 shadow-xs ${
            isPill ? 'rounded-full px-3.5' : isMac ? 'rounded-md px-2.5' : 'rounded'
          }`}
          title="Open Application Launcher (Super / Alt+Space)"
        >
          <span className="font-mono text-xs">{isMac ? ' OXIDE' : 'OXIDE'}</span>
        </button>

        {/* Active Window Title */}
        {activeWindowTitle && (
          <div
            style={{ borderColor: theme.palette.border }}
            className="hidden md:flex items-center gap-2 pl-2 border-l text-[11px]"
          >
            <span style={{ color: theme.palette.textPrimary }} className="truncate max-w-[200px] font-medium">
              {activeWindowTitle}
            </span>
          </div>
        )}
      </div>

      {/* Center Zone: System Clock & Date */}
      <div className="flex items-center gap-2 font-medium" style={{ color: theme.palette.textPrimary }}>
        <span style={{ color: theme.palette.textSecondary }}>{dateStr}</span>
        <span style={{ color: theme.palette.border }}>·</span>
        <span className="font-mono font-semibold">{timeStr}</span>
      </div>

      {/* Right Zone: Quick Settings Trigger */}
      <div className="flex items-center gap-2">
        {/* Quick Settings Drawer Toggle */}
        <button
          onClick={onToggleQuickSettings}
          style={
            quickSettingsOpen
              ? { backgroundColor: theme.palette.surface, color: theme.palette.textPrimary }
              : { color: theme.palette.textSecondary }
          }
          className={`flex items-center gap-1.5 px-2 py-1 transition-colors hover:opacity-80 ${
            isPill ? 'rounded-full' : 'rounded'
          }`}
          aria-pressed={quickSettingsOpen}
          title="Quick Settings & Hardware Tuner"
        >
          {quickSettingsOpen ? (
            <ToggleRight className="w-4 h-4" />
          ) : (
            <ToggleLeft className="w-4 h-4" />
          )}
        </button>
      </div>
    </header>
  );
}
