'use client';

import React, { useState, useRef, useEffect } from 'react';
import { WindowState, DEConfig } from '@/lib/oxide-types';
import { Minus, Square, X, Copy, Maximize2 } from 'lucide-react';

interface WindowFrameProps {
  window: WindowState;
  isActive: boolean;
  config: DEConfig;
  onFocus: () => void;
  onClose: () => void;
  onMinimize: () => void;
  onToggleMaximize: () => void;
  onUpdatePosition: (x: number, y: number) => void;
  onUpdateSize: (w: number, h: number) => void;
  onTitleContextMenu: (event: React.MouseEvent) => void;
  children: React.ReactNode;
}

export default function WindowFrame({
  window: win,
  isActive,
  config,
  onFocus,
  onClose,
  onMinimize,
  onToggleMaximize,
  onUpdatePosition,
  onUpdateSize,
  onTitleContextMenu,
  children,
}: WindowFrameProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [isResizing, setIsResizing] = useState(false);
  const dragStartRef = useRef<{ mouseX: number; mouseY: number; winX: number; winY: number }>({
    mouseX: 0,
    mouseY: 0,
    winX: win.x,
    winY: win.y,
  });
  const resizeStartRef = useRef<{ mouseX: number; mouseY: number; winW: number; winH: number }>({
    mouseX: 0,
    mouseY: 0,
    winW: win.width,
    winH: win.height,
  });

  // Handle Dragging
  const handleTitleMouseDown = (e: React.MouseEvent) => {
    if (e.button !== 0 || win.isMaximized || config.tilingMode) return;
    onFocus();
    setIsDragging(true);
    dragStartRef.current = {
      mouseX: e.clientX,
      mouseY: e.clientY,
      winX: win.x,
      winY: win.y,
    };
  };

  // Handle Resizing
  const handleResizeMouseDown = (e: React.MouseEvent) => {
    if (e.button !== 0 || win.isMaximized || config.tilingMode) return;
    e.stopPropagation();
    onFocus();
    setIsResizing(true);
    resizeStartRef.current = {
      mouseX: e.clientX,
      mouseY: e.clientY,
      winW: win.width,
      winH: win.height,
    };
  };

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (isDragging) {
        const dx = e.clientX - dragStartRef.current.mouseX;
        const dy = e.clientY - dragStartRef.current.mouseY;
        const newX = Math.max(0, dragStartRef.current.winX + dx);
        const newY = Math.max(36, dragStartRef.current.winY + dy);
        onUpdatePosition(newX, newY);
      } else if (isResizing) {
        const dx = e.clientX - resizeStartRef.current.mouseX;
        const dy = e.clientY - resizeStartRef.current.mouseY;
        const newW = Math.max(320, resizeStartRef.current.winW + dx);
        const newH = Math.max(220, resizeStartRef.current.winH + dy);
        onUpdateSize(newW, newH);
      }
    };

    const handleMouseUp = () => {
      setIsDragging(false);
      setIsResizing(false);
    };

    if (isDragging || isResizing) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    }

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, isResizing, onUpdatePosition, onUpdateSize]);

  if (win.isMinimized) return null;

  // Compute style depending on maximized or tiling
  const style: React.CSSProperties = win.isMaximized
    ? {
        position: 'absolute',
        top: 36,
        left: 0,
        right: 0,
        bottom: 0,
        width: '100%',
        height: 'calc(100% - 36px)',
        zIndex: win.zIndex,
      }
    : config.tilingMode
    ? {
        position: 'relative',
        width: '100%',
        height: '100%',
        zIndex: win.zIndex,
      }
    : {
        position: 'absolute',
        left: `${win.x}px`,
        top: `${win.y}px`,
        width: `${win.width}px`,
        height: `${win.height}px`,
        zIndex: win.zIndex,
      };

  const theme = config.activeTheme;
  const isLeftControls = theme.windowStyle.windowControls === 'left';
  const titlebarHeight = theme.windowStyle.compactTitlebar ? '28px' : '34px';
  const isMac = theme.windowStyle.isMacStyle || isLeftControls;
  const isPill = theme.windowStyle.isPillShape;

  return (
    <div
      style={{
        ...style,
        backgroundColor: theme.palette.surface,
        borderColor: theme.palette.border,
        borderRadius: win.isMaximized || config.tilingMode ? 0 : `${theme.windowStyle.borderRadius}px`,
        borderWidth: `${theme.windowStyle.borderWidth}px`,
        boxShadow: config.lowSpecMode || !theme.windowStyle.backdropBlur
          ? 'none'
          : isActive
          ? '0 20px 25px -5px rgba(0, 0, 0, 0.55)'
          : '0 10px 15px -3px rgba(0, 0, 0, 0.3)',
        fontFamily: theme.fontFamily === 'System Sans' ? 'inherit' : theme.fontFamily,
      }}
      onMouseDown={onFocus}
      data-window-frame=""
      onContextMenu={(event) => event.stopPropagation()}
      className={`flex flex-col overflow-hidden select-none border transition-opacity duration-[1000ms] ease-out ${
        win.isClosing ? 'pointer-events-none opacity-0' : 'opacity-100'
      }`}
    >
      {/* Title Bar */}
      <div
        onMouseDown={handleTitleMouseDown}
        onDoubleClick={onToggleMaximize}
        onContextMenu={(event) => {
          event.preventDefault();
          event.stopPropagation();
          onTitleContextMenu(event);
        }}
        style={{
          height: titlebarHeight,
          backgroundColor: theme.palette.panel,
          borderColor: theme.palette.border,
        }}
        className={`px-3 flex items-center justify-between border-b shrink-0 cursor-default relative`}
      >
        {/* macOS-style Traffic Lights on Left */}
        {isMac ? (
          <div className="flex items-center gap-2 shrink-0 z-10" onMouseDown={(e) => e.stopPropagation()}>
            <button
              onClick={onClose}
              className="w-3 h-3 rounded-full bg-[#ff5f56] hover:bg-[#ff5f56]/90 border border-[#e0443e] flex items-center justify-center transition-transform hover:scale-105"
              title="Close"
            >
              <X className="w-2 h-2 text-[#4d0000] opacity-0 hover:opacity-100 transition-opacity" />
            </button>
            <button
              onClick={onMinimize}
              className="w-3 h-3 rounded-full bg-[#ffbd2e] hover:bg-[#ffbd2e]/90 border border-[#dea123] flex items-center justify-center transition-transform hover:scale-105"
              title="Minimize"
            >
              <Minus className="w-2 h-2 text-[#5c3c00] opacity-0 hover:opacity-100 transition-opacity" />
            </button>
            <button
              onClick={onToggleMaximize}
              className="w-3 h-3 rounded-full bg-[#27c93f] hover:bg-[#27c93f]/90 border border-[#1aab29] flex items-center justify-center transition-transform hover:scale-105"
              title={win.isMaximized ? 'Restore' : 'Zoom'}
            >
              <Maximize2 className="w-2 h-2 text-[#004d00] opacity-0 hover:opacity-100 transition-opacity" />
            </button>
          </div>
        ) : (
          <div className="flex items-center gap-2 min-w-0 z-10">
            <div
              className={`w-2.5 h-2.5 shrink-0 ${isPill ? 'rounded-full' : 'rounded-xs'}`}
              style={{ backgroundColor: isActive ? theme.palette.accent : theme.palette.textSecondary }}
            />
            <span
              style={{ color: isActive ? theme.palette.textPrimary : theme.palette.textSecondary }}
              className="font-medium text-xs truncate select-none"
            >
              {win.title}
            </span>
          </div>
        )}

        {/* Centered Title for macOS-style */}
        {isMac && (
          <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
            <span
              style={{ color: isActive ? theme.palette.textPrimary : theme.palette.textSecondary }}
              className="font-medium text-xs truncate max-w-[240px] select-none"
            >
              {win.title}
            </span>
          </div>
        )}

        {/* Window Controls on Right (for non-macOS layouts) */}
        {!isMac && (
          <div
            className={`flex items-center gap-1.5 shrink-0 z-10 ${
              isPill ? 'px-2 py-0.5 rounded-full border border-neutral-800' : ''
            }`}
            onMouseDown={(e) => e.stopPropagation()}
          >
            <button
              onClick={onMinimize}
              className={`w-5 h-5 flex items-center justify-center transition-colors hover:bg-neutral-800 text-neutral-400 hover:text-white ${
                isPill ? 'rounded-full' : 'rounded'
              }`}
              title="Minimize"
            >
              <Minus className="w-3 h-3" />
            </button>
            <button
              onClick={onToggleMaximize}
              className={`w-5 h-5 flex items-center justify-center transition-colors hover:bg-neutral-800 text-neutral-400 hover:text-white ${
                isPill ? 'rounded-full' : 'rounded'
              }`}
              title={win.isMaximized ? 'Restore' : 'Maximize'}
            >
              <Maximize2 className="w-3 h-3" />
            </button>
            <button
              onClick={onClose}
              className={`w-5 h-5 flex items-center justify-center transition-colors hover:bg-rose-600/80 text-neutral-400 hover:text-white ${
                isPill ? 'rounded-full' : 'rounded'
              }`}
              title="Close"
            >
              <X className="w-3 h-3" />
            </button>
          </div>
        )}
      </div>

      {/* Window Content */}
      <div className="relative min-h-0 flex-1 overflow-auto">{children}</div>

      {/* Resize Handle (Bottom-Right) */}
      {!win.isMaximized && !config.tilingMode && (
        <div
          onMouseDown={handleResizeMouseDown}
          className="absolute bottom-0 right-0 w-4 h-4 cursor-se-resize z-20 flex items-end justify-end p-0.5 text-neutral-600 hover:text-neutral-400"
        >
          <svg className="w-2.5 h-2.5" viewBox="0 0 6 6" fill="currentColor">
            <circle cx="5" cy="5" r="1" />
            <circle cx="5" cy="2" r="1" />
            <circle cx="2" cy="5" r="1" />
          </svg>
        </div>
      )}
    </div>
  );
}
