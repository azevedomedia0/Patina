'use client';

import React, { useState } from 'react';
import { Folder, FileText, FileCode, HardDrive, Search, ChevronRight, Download, Eye } from 'lucide-react';

interface FileItem {
  name: string;
  type: 'dir' | 'file';
  size: string;
  modified: string;
  perm: string;
  content?: string;
}

const FILE_SYSTEM: Record<string, FileItem[]> = {
  '/home/oxide': [
    { name: '.config', type: 'dir', size: '4.0 KB', modified: 'Oct 13 18:30', perm: 'drwxr-xr-x' },
    { name: 'src', type: 'dir', size: '4.0 KB', modified: 'Oct 13 18:15', perm: 'drwxr-xr-x' },
    { name: 'Cargo.toml', type: 'file', size: '542 B', modified: 'Oct 13 18:22', perm: '-rw-r--r--', content: '[workspace]\nmembers = ["crates/oxide-compositor", "crates/oxide-panel", "crates/oxide-launcher"]\nresolver = "2"\n\n[profile.release]\nopt-level = 3\nlto = "fat"\npanic = "abort"' },
    { name: 'build-arm.sh', type: 'file', size: '820 B', modified: 'Oct 13 18:25', perm: '-rwxr-xr-x', content: '#!/bin/bash\n# Cross compile for armv7l and aarch64\ncargo build --release --target armv7-unknown-linux-gnueabihf' },
    { name: 'README.md', type: 'file', size: '1.2 KB', modified: 'Oct 13 18:10', perm: '-rw-r--r--', content: '# OxideDE\nUltra-lightweight Linux Desktop Environment written in pure Rust.\n\n- Wayland native (Smithay)\n- Dual OpenGL ES & tiny-skia software renderers\n- Optimized for 32-bit & 64-bit ARM' },
  ],
  '/home/oxide/.config': [
    { name: 'oxide', type: 'dir', size: '4.0 KB', modified: 'Oct 13 18:28', perm: 'drwxr-xr-x' },
    { name: 'alacritty', type: 'dir', size: '4.0 KB', modified: 'Oct 13 17:40', perm: 'drwxr-xr-x' },
  ],
  '/home/oxide/.config/oxide': [
    { name: 'config.ron', type: 'file', size: '614 B', modified: 'Oct 13 18:28', perm: '-rw-r--r--', content: '(\n  theme: "oxide-dark",\n  low_spec_mode: false,\n  direct_scanout: true,\n  damage_tracking: true,\n  tiling_mode: false,\n  font_family: "JetBrains Mono",\n  font_size: 13.0,\n)' },
    { name: 'theme.toml', type: 'file', size: '380 B', modified: 'Oct 13 18:20', perm: '-rw-r--r--', content: '[palette]\nbackground = "#0d1117"\nforeground = "#c9d1d9"\naccent = "#f59e0b"\nborder = "#30363d"' },
  ],
  '/home/oxide/src': [
    { name: 'oxide-compositor', type: 'dir', size: '4.0 KB', modified: 'Oct 13 18:15', perm: 'drwxr-xr-x' },
    { name: 'oxide-panel', type: 'dir', size: '4.0 KB', modified: 'Oct 13 18:16', perm: 'drwxr-xr-x' },
    { name: 'oxide-launcher', type: 'dir', size: '4.0 KB', modified: 'Oct 13 18:17', perm: 'drwxr-xr-x' },
  ],
};

export default function FileManagerApp() {
  const [currentPath, setCurrentPath] = useState<string>('/home/oxide');
  const [selectedFile, setSelectedFile] = useState<FileItem | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  const items = (FILE_SYSTEM[currentPath] || []).filter((item) =>
    item.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleNavigate = (dirName: string) => {
    let nextPath = currentPath === '/' ? `/${dirName}` : `${currentPath}/${dirName}`;
    if (FILE_SYSTEM[nextPath]) {
      setCurrentPath(nextPath);
      setSelectedFile(null);
    }
  };

  const handleGoUp = () => {
    if (currentPath === '/home/oxide') return;
    const parts = currentPath.split('/').filter(Boolean);
    parts.pop();
    const upPath = '/' + parts.join('/');
    if (FILE_SYSTEM[upPath]) {
      setCurrentPath(upPath);
      setSelectedFile(null);
    } else {
      setCurrentPath('/home/oxide');
      setSelectedFile(null);
    }
  };

  return (
    <div className="flex flex-col h-full bg-neutral-950 text-neutral-200 font-sans select-none text-xs">
      {/* Top Toolbar */}
      <div className="flex items-center justify-between px-3 py-2 bg-neutral-900 border-b border-neutral-800 gap-2">
        <div className="flex items-center gap-1.5 flex-1 overflow-x-auto">
          <button
            onClick={handleGoUp}
            disabled={currentPath === '/home/oxide'}
            className="px-2 py-1 bg-neutral-800 hover:bg-neutral-700 disabled:opacity-40 disabled:hover:bg-neutral-800 rounded text-neutral-300 font-mono transition-colors"
          >
            ↑ Up
          </button>
          <div className="flex items-center gap-1 px-2 py-1 bg-neutral-950 rounded border border-neutral-800 font-mono text-neutral-300">
            <HardDrive className="w-3.5 h-3.5 text-neutral-400 shrink-0" />
            <span>{currentPath}</span>
          </div>
        </div>

        <div className="flex items-center gap-1.5 px-2 py-1 bg-neutral-950 rounded border border-neutral-800 w-40">
          <Search className="w-3.5 h-3.5 text-neutral-400 shrink-0" />
          <input
            type="text"
            placeholder="Filter files..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-transparent text-xs text-white outline-none placeholder:text-neutral-500"
          />
        </div>
      </div>

      {/* Main Body: File List + Preview Pane */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left: Quick Access Side Menu */}
        <div className="w-36 bg-neutral-900/60 border-r border-neutral-800 p-2 space-y-1 shrink-0 hidden sm:block">
          <div className="text-[10px] text-neutral-400 uppercase tracking-wider px-2 py-1 font-semibold">Bookmarks</div>
          <button
            onClick={() => { setCurrentPath('/home/oxide'); setSelectedFile(null); }}
            className={`w-full text-left px-2 py-1.5 rounded flex items-center gap-2 ${currentPath === '/home/oxide' ? 'bg-amber-500/20 text-amber-300' : 'text-neutral-400 hover:text-white hover:bg-neutral-800'}`}
          >
            <Folder className="w-3.5 h-3.5" />
            <span>Home</span>
          </button>
          <button
            onClick={() => { setCurrentPath('/home/oxide/.config/oxide'); setSelectedFile(null); }}
            className={`w-full text-left px-2 py-1.5 rounded flex items-center gap-2 ${currentPath === '/home/oxide/.config/oxide' ? 'bg-amber-500/20 text-amber-300' : 'text-neutral-400 hover:text-white hover:bg-neutral-800'}`}
          >
            <Folder className="w-3.5 h-3.5" />
            <span>Oxide Config</span>
          </button>
          <button
            onClick={() => { setCurrentPath('/home/oxide/src'); setSelectedFile(null); }}
            className={`w-full text-left px-2 py-1.5 rounded flex items-center gap-2 ${currentPath === '/home/oxide/src' ? 'bg-amber-500/20 text-amber-300' : 'text-neutral-400 hover:text-white hover:bg-neutral-800'}`}
          >
            <FileCode className="w-3.5 h-3.5" />
            <span>Rust Source</span>
          </button>
        </div>

        {/* Center: Files Grid / List */}
        <div className="flex-1 p-3 overflow-y-auto">
          {items.length === 0 ? (
            <div className="text-neutral-500 text-center py-12">No files match &quot;{searchQuery}&quot;</div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-2">
              {items.map((item) => (
                <div
                  key={item.name}
                  onClick={() => {
                    if (item.type === 'dir') {
                      handleNavigate(item.name);
                    } else {
                      setSelectedFile(item);
                    }
                  }}
                  className={`p-2.5 rounded-lg border cursor-pointer transition-all flex items-center gap-2.5 ${
                    selectedFile?.name === item.name
                      ? 'bg-neutral-800 border-amber-500/50 text-white shadow-sm'
                      : 'bg-neutral-900/80 border-neutral-800 hover:bg-neutral-800/60 hover:border-neutral-700'
                  }`}
                >
                  {item.type === 'dir' ? (
                    <Folder className="w-5 h-5 text-amber-400 shrink-0" />
                  ) : item.name.endsWith('.rs') || item.name.endsWith('.toml') ? (
                    <FileCode className="w-5 h-5 text-cyan-400 shrink-0" />
                  ) : (
                    <FileText className="w-5 h-5 text-neutral-400 shrink-0" />
                  )}
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-neutral-200 truncate">{item.name}</div>
                    <div className="text-[10px] text-neutral-400 font-mono flex items-center gap-2">
                      <span>{item.size}</span>
                      <span>·</span>
                      <span>{item.perm}</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Right: File Preview Sidebar if file selected */}
        {selectedFile && (
          <div className="w-64 bg-neutral-900 border-l border-neutral-800 p-3 flex flex-col space-y-2 shrink-0 overflow-y-auto">
            <div className="flex items-center justify-between pb-2 border-b border-neutral-800">
              <span className="font-semibold text-white truncate">{selectedFile.name}</span>
              <button
                onClick={() => setSelectedFile(null)}
                className="text-neutral-400 hover:text-white"
              >
                ✕
              </button>
            </div>
            <div className="space-y-1 text-[11px] text-neutral-400">
              <div>Size: <span className="text-white font-mono">{selectedFile.size}</span></div>
              <div>Permissions: <span className="text-emerald-400 font-mono">{selectedFile.perm}</span></div>
              <div>Modified: <span className="text-neutral-300 font-mono">{selectedFile.modified}</span></div>
            </div>

            {selectedFile.content && (
              <div className="flex-1 flex flex-col space-y-1 pt-2">
                <span className="text-[10px] uppercase text-neutral-400 font-semibold tracking-wider">Preview</span>
                <pre className="flex-1 p-2 bg-neutral-950 rounded border border-neutral-800 text-[10px] font-mono text-neutral-300 overflow-x-auto whitespace-pre-wrap">
                  {selectedFile.content}
                </pre>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
