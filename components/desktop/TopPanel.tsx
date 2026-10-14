'use client';

import React, { useState, useEffect } from 'react';
import { DEConfig, AppDefinition, AppId } from '@/lib/oxide-types';
import { Terminal, Cpu, HardDrive, Wifi, Volume2, ToggleLeft, ToggleRight, LayoutGrid, Monitor, ShieldCheck } from 'lucide-react';

interface TopPanelProps {
  config: DEConfig;
  onToggleLauncher: () => void;
  onToggleQuickSettings: () => void;
  quickSettingsOpen: boolean;
  onToggleNotifications: () => void;
  notificationsOpen: boolean;
  onToggleWorkspaces: () => void;
  workspacesOpen: boolean;
  onOpenApp: (appId: AppId) => void;
  onContextMenu: (event: React.MouseEvent) => void;
}

export default function TopPanel({
  config,
  onToggleLauncher,
  onToggleQuickSettings,
  quickSettingsOpen,
  onToggleNotifications,
  notificationsOpen,
  onToggleWorkspaces,
  workspacesOpen,
  onOpenApp,
  onContextMenu,
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

  return (
    <header
      style={{
        backgroundColor: theme.palette.panel,
        borderColor: theme.palette.border,
        color: theme.palette.textSecondary,
        fontFamily: theme.fontFamily === 'System Sans' ? 'inherit' : theme.fontFamily,
      }}
      className="h-9 w-full backdrop-blur-md border-b px-2 sm:px-3 flex items-center justify-between text-xs select-none z-50 shrink-0 transition-colors duration-200"
      onContextMenu={onContextMenu}
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
          className="flex items-center gap-1.5 rounded-full px-2.5 py-1 font-bold text-xs transition-opacity hover:opacity-90 shadow-xs"
          aria-label="Apps"
          title="Open Application Launcher (Super / Alt+Space)"
        >
          <LayoutGrid className="w-3.5 h-3.5" />
        </button>

        <button
          type="button"
          onClick={onToggleWorkspaces}
          aria-expanded={workspacesOpen}
          aria-label="Workspaces"
          title="Workspaces"
          style={{
            borderColor: theme.palette.border,
            color: theme.palette.textPrimary,
            backgroundColor: 'transparent',
          }}
          className="hidden md:flex items-center gap-2 rounded-md border-l py-1 pl-2 pr-2 text-[11px] font-medium transition-colors hover:opacity-80"
        >
          Workspace
        </button>
      </div>

      {/* Center Zone: System Clock & Date */}
      <button
        type="button"
        onClick={onToggleNotifications}
        aria-expanded={notificationsOpen}
        aria-label="Notifications"
        title="Notifications"
        style={{
          color: theme.palette.textPrimary,
          backgroundColor: notificationsOpen ? theme.palette.surface : 'transparent',
        }}
        className="flex items-center gap-2 rounded-md px-2 py-1 font-medium transition-colors hover:opacity-80"
      >
        <span style={{ color: theme.palette.textSecondary }}>{dateStr}</span>
        <span style={{ color: theme.palette.border }}>·</span>
        <span className="font-mono font-semibold">{timeStr}</span>
      </button>

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
