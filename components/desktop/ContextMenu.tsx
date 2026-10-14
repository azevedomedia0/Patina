'use client';

import React, { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { DynamicTheme } from '@/lib/oxide-types';

export type ContextMenuEntry =
  | { kind: 'item'; id: string; label: string; danger?: boolean; disabled?: boolean }
  | { kind: 'separator' };

export default function ContextMenu({
  x,
  y,
  entries,
  theme,
  onSelect,
  onClose,
}: {
  x: number;
  y: number;
  entries: ContextMenuEntry[];
  theme: DynamicTheme;
  onSelect: (id: string) => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const pad = 8;
    const nextX = x + rect.width > window.innerWidth - pad ? Math.max(pad, window.innerWidth - rect.width - pad) : x;
    const nextY = y + rect.height > window.innerHeight - pad ? Math.max(pad, window.innerHeight - rect.height - pad) : y;
    setPos({ x: nextX, y: nextY });
  }, [x, y, entries]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    const onPointer = (event: PointerEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) onClose();
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('pointerdown', onPointer);
    window.addEventListener('scroll', onClose, true);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('pointerdown', onPointer);
      window.removeEventListener('scroll', onClose, true);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      role="menu"
      style={{
        left: pos.x,
        top: pos.y,
        backgroundColor: theme.palette.panel,
        borderColor: theme.palette.border,
        color: theme.palette.textPrimary,
        fontFamily: theme.fontFamily === 'System Sans' ? 'inherit' : theme.fontFamily,
      }}
      className="fixed z-[80] min-w-44 rounded-xl border py-1 shadow-[0_16px_48px_rgba(0,0,0,0.45)] select-none"
      onContextMenu={(event) => event.preventDefault()}
    >
      {entries.map((entry, index) =>
        entry.kind === 'separator' ? (
          <div key={`sep-${index}`} role="separator" className="my-1 h-px" style={{ backgroundColor: theme.palette.border }} />
        ) : (
          <button
            key={entry.id}
            type="button"
            role="menuitem"
            disabled={entry.disabled}
            onClick={() => {
              if (entry.disabled) return;
              onSelect(entry.id);
              onClose();
            }}
            style={{ color: entry.danger ? theme.palette.danger : theme.palette.textPrimary }}
            className="flex w-full items-center px-3 py-1.5 text-left text-[13px] enabled:hover:bg-white/10 disabled:opacity-40"
          >
            {entry.label}
          </button>
        ),
      )}
    </div>
  );
}
