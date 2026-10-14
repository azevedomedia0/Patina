'use client';

import React, { useState, useEffect, useRef } from 'react';
import { AppDefinition, AppId } from '@/lib/oxide-types';
import { Search, Terminal, Activity, Folder, FileCode, Gauge, Settings, BookOpen, X, Palette, Cpu } from 'lucide-react';

interface AppLauncherProps {
  isOpen: boolean;
  onClose: () => void;
  apps: AppDefinition[];
  onLaunchApp: (appId: AppId) => void;
}

export default function AppLauncher({ isOpen, onClose, apps, onLaunchApp }: AppLauncherProps) {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) {
      inputRef.current?.focus();
    }
  }, [isOpen]);

  const filteredApps = apps.filter((app) =>
    app.title.toLowerCase().includes(query.toLowerCase()) ||
    app.description.toLowerCase().includes(query.toLowerCase()) ||
    app.category.toLowerCase().includes(query.toLowerCase())
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((idx) => (idx + 1) % Math.max(1, filteredApps.length));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((idx) => (idx - 1 + filteredApps.length) % Math.max(1, filteredApps.length));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filteredApps[selectedIndex]) {
        onLaunchApp(filteredApps[selectedIndex].id);
        onClose();
      }
    }
  };

  if (!isOpen) return null;

  const getIcon = (id: AppId) => {
    switch (id) {
      case 'terminal': return <Terminal className="w-5 h-5 text-amber-400" />;
      case 'sysmon': return <Activity className="w-5 h-5 text-emerald-400" />;
      case 'theme-studio': return <Palette className="w-5 h-5 text-purple-400" />;
      case 'profiler': return <Cpu className="w-5 h-5 text-cyan-400" />;
      case 'files': return <Folder className="w-5 h-5 text-cyan-400" />;
      case 'codebase': return <FileCode className="w-5 h-5 text-purple-400" />;
      case 'benchmark': return <Gauge className="w-5 h-5 text-rose-400" />;
      case 'settings': return <Settings className="w-5 h-5 text-blue-400" />;
      case 'guide': return <BookOpen className="w-5 h-5 text-amber-300" />;
      default: return <Terminal className="w-5 h-5 text-neutral-400" />;
    }
  };

  return (
    <div
      className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-start justify-center pt-24 z-50 p-4"
      onClick={onClose}
    >
      <div
        className="w-full max-w-lg bg-neutral-900 border border-neutral-800 rounded-xl shadow-2xl overflow-hidden flex flex-col text-xs text-neutral-200"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        {/* Search Input Bar */}
        <div className="flex items-center gap-2.5 px-4 py-3 border-b border-neutral-800 bg-neutral-950/70">
          <Search className="w-4 h-4 text-neutral-400 shrink-0" />
          <input
            ref={inputRef}
            type="text"
            placeholder="Type application name or keyword..."
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            className="flex-1 bg-transparent text-sm text-white outline-none placeholder:text-neutral-500 font-sans"
          />
          <button
            onClick={onClose}
            className="text-neutral-500 hover:text-white p-1 rounded hover:bg-neutral-800"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Apps List */}
        <div className="max-h-80 overflow-y-auto p-2 space-y-1">
          {filteredApps.length === 0 ? (
            <div className="p-8 text-center text-neutral-500">No applications found matching &quot;{query}&quot;</div>
          ) : (
            filteredApps.map((app, idx) => {
              const isSelected = idx === selectedIndex;
              return (
                <div
                  key={app.id}
                  onClick={() => {
                    onLaunchApp(app.id);
                    onClose();
                  }}
                  onMouseEnter={() => setSelectedIndex(idx)}
                  className={`p-2.5 rounded-lg cursor-pointer flex items-center gap-3 transition-colors ${
                    isSelected
                      ? 'bg-neutral-800 text-white border border-amber-500/40 shadow-xs'
                      : 'hover:bg-neutral-850 text-neutral-300'
                  }`}
                >
                  <div className="p-2 rounded-lg bg-neutral-950 border border-neutral-800 shrink-0">
                    {getIcon(app.id)}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-semibold text-xs text-white flex items-center justify-between">
                      <span>{app.title}</span>
                      <span className="text-[10px] text-neutral-500 font-mono">{app.category}</span>
                    </div>
                    <div className="text-[11px] text-neutral-400 truncate">{app.description}</div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Footer info */}
        <div className="px-4 py-2 bg-neutral-950 border-t border-neutral-800 text-[10px] text-neutral-400 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span>↑↓ Navigate</span>
            <span>·</span>
            <span>↵ Launch</span>
            <span>·</span>
            <span>Esc Close</span>
          </div>
          <span className="font-mono text-amber-400">OxideDE Wayland</span>
        </div>
      </div>
    </div>
  );
}
