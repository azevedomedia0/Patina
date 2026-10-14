'use client';

import React, { useState } from 'react';
import { RUST_CODEBASE, RustFile } from '@/lib/oxide-rust-codebase';
import { FileCode, Copy, Check, Download, Layers, ShieldCheck, Cpu, Terminal, Sparkles } from 'lucide-react';
import JSZip from 'jszip';

export default function RustCodebaseApp() {
  const [activeFileIndex, setActiveFileIndex] = useState(0);
  const [copied, setCopied] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [selectedCategory, setSelectedCategory] = useState<string>('All');

  const activeFile = RUST_CODEBASE[activeFileIndex] || RUST_CODEBASE[0];

  const categories = [
    'All',
    'Compositor',
    'Optimizations & ARM SIMD',
    'Theming Engine',
    'Panel & UI',
    'Launcher',
    'Config',
    'Build & Cross-Compile',
    'Packaging',
  ];

  const filteredFiles = selectedCategory === 'All'
    ? RUST_CODEBASE
    : RUST_CODEBASE.filter((f) => f.category === selectedCategory);

  const handleCopy = () => {
    navigator.clipboard.writeText(activeFile.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleDownloadZip = async () => {
    setDownloading(true);
    try {
      const zip = new JSZip();

      // Add each file to the zip with appropriate folders
      RUST_CODEBASE.forEach((file) => {
        zip.file(file.path, file.content);
      });

      // Add a helpful README.md
      zip.file(
        'README.md',
        `# OxideDE - Lightweight Rust Linux Desktop Environment\n\n` +
        `This is the complete source repository for OxideDE, designed for low-end hardware, 32-bit ARM (armv7l), and 64-bit ARM (aarch64).\n\n` +
        `## Quick Compilation:\n` +
        `\`\`\`bash\n` +
        `# Install Rust & build dependencies\n` +
        `sudo apt-get install -y libwayland-dev libxkbcommon-dev libseat-dev libinput-dev libpixman-1-dev\n` +
        `cargo build --release\n` +
        `\`\`\`\n\n` +
        `## Cross-Compiling for Raspberry Pi 2/3 (32-bit ARM):\n` +
        `\`\`\`bash\n` +
        `./scripts/build-armv7.sh\n` +
        `\`\`\`\n`
      );

      const blob = await zip.generateAsync({ type: 'blob' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'oxidede-rust-source.zip';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Failed to generate ZIP:', err);
    } finally {
      setDownloading(false);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col bg-neutral-950 font-sans text-xs text-neutral-200 select-none">
      {/* Top Header & Actions */}
      <div className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-b border-neutral-800 bg-neutral-900 px-4 py-2.5">
        <div>
          <div className="flex items-center gap-2">
            <span className="font-semibold text-white text-sm">OxideDE Rust Workspace Studio</span>
            <span className="text-emerald-400 font-mono text-[11px]">v0.9.4</span>
          </div>
          <p className="text-[11px] text-neutral-400">
            Real production-grade Rust codebase ready to compile for x86_64, aarch64, and armv7l
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleCopy}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-neutral-800 hover:bg-neutral-700 text-neutral-200 rounded border border-neutral-700 font-medium transition-colors"
          >
            {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5 text-neutral-400" />}
            <span>{copied ? 'Copied' : 'Copy File'}</span>
          </button>

          <button
            onClick={handleDownloadZip}
            disabled={downloading}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-amber-500 hover:bg-amber-400 text-neutral-950 font-semibold rounded transition-colors shadow-sm disabled:opacity-50"
          >
            <Download className="w-3.5 h-3.5" />
            <span>{downloading ? 'Packing ZIP...' : 'Download Full Rust Project (.zip)'}</span>
          </button>
        </div>
      </div>

      {/* Category Filter Bar */}
      <div className="flex shrink-0 items-center gap-1 overflow-x-auto border-b border-neutral-800 bg-neutral-900/60 px-4 py-1.5">
        <span className="text-[10px] text-neutral-400 uppercase font-semibold mr-1">Modules:</span>
        {categories.map((cat) => (
          <button
            key={cat}
            onClick={() => setSelectedCategory(cat)}
            className={`px-2.5 py-1 rounded text-[11px] whitespace-nowrap transition-colors ${
              selectedCategory === cat
                ? 'bg-neutral-800 text-amber-400 font-semibold shadow-xs'
                : 'text-neutral-400 hover:text-white hover:bg-neutral-850'
            }`}
          >
            {cat}
          </button>
        ))}
      </div>

      {/* Main Split Layout: File Tree + Code Viewer */}
      <div className="flex min-h-0 flex-1 overflow-hidden">
        {/* Left: Files List */}
        <div className="flex w-64 shrink-0 flex-col border-r border-neutral-800 bg-neutral-900/40 min-h-0">
          <div className="p-2 text-[10px] uppercase font-semibold text-neutral-400 tracking-wider">
            Workspace Files ({filteredFiles.length})
          </div>
          <div className="min-h-0 flex-1 space-y-1 overflow-y-auto p-1.5">
            {filteredFiles.map((file) => {
              const originalIndex = RUST_CODEBASE.indexOf(file);
              const isSelected = originalIndex === activeFileIndex;
              return (
                <button
                  key={file.path}
                  onClick={() => setActiveFileIndex(originalIndex)}
                  className={`w-full text-left p-2 rounded-lg transition-all flex flex-col gap-0.5 ${
                    isSelected
                      ? 'bg-neutral-800 border border-amber-500/40 text-white'
                      : 'hover:bg-neutral-850 text-neutral-300'
                  }`}
                >
                  <div className="flex items-center gap-1.5">
                    <FileCode className={`w-3.5 h-3.5 shrink-0 ${isSelected ? 'text-amber-400' : 'text-neutral-400'}`} />
                    <span className="font-mono text-[11px] truncate">{file.filename}</span>
                  </div>
                  <span className="text-[10px] text-neutral-400 truncate pl-5">{file.path}</span>
                </button>
              );
            })}
          </div>

          {/* Architecture Insights Box */}
          <div className="p-3 border-t border-neutral-800 bg-neutral-900/80 space-y-1.5 text-[11px]">
            <div className="font-semibold text-white flex items-center gap-1.5">
              <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
              <span>Rust Safety & Efficiency</span>
            </div>
            <p className="text-neutral-400 text-[10px] leading-relaxed">
              No GC pauses, no segfaults. Uses <code className="text-neutral-300">smithay</code> + <code className="text-neutral-300">tiny-skia</code> fallback for low-spec 32-bit ARM SBCs.
            </p>
          </div>
        </div>

        {/* Right: Code Viewer */}
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden bg-neutral-950">
          {/* File Meta Header */}
          <div className="px-4 py-2 border-b border-neutral-800 bg-neutral-900/50 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="font-mono text-emerald-400 font-semibold">{activeFile.path}</span>
              <span className="text-neutral-500">·</span>
              <span className="text-neutral-400 text-[11px]">{activeFile.description}</span>
            </div>
            <span className="px-2 py-0.5 rounded bg-neutral-800 text-[10px] font-mono text-neutral-300 uppercase">
              {activeFile.language}
            </span>
          </div>

          {/* Code Text Content */}
          <div className="min-h-0 flex-1 overflow-auto p-4 font-mono text-xs leading-relaxed text-neutral-200 select-text">
            <pre className="whitespace-pre">{activeFile.content}</pre>
          </div>
        </div>
      </div>
    </div>
  );
}
