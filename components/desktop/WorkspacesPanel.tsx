'use client';

import React from 'react';
import { DEConfig } from '@/lib/oxide-types';

const WORKSPACES = [1, 2, 3, 4] as const;

interface WorkspacesPanelProps {
  isOpen: boolean;
  config: DEConfig;
  currentWorkspace: number;
  onSelectWorkspace: (workspace: number) => void;
}

export default function WorkspacesPanel({
  isOpen,
  config,
  currentWorkspace,
  onSelectWorkspace,
}: WorkspacesPanelProps) {
  if (!isOpen) return null;

  const theme = config.activeTheme;

  return (
    <div
      role="dialog"
      aria-label="Workspaces"
      style={{
        backgroundColor: theme.palette.panel,
        borderColor: theme.palette.border,
        color: theme.palette.textPrimary,
        fontFamily: theme.fontFamily === 'System Sans' ? 'inherit' : theme.fontFamily,
      }}
      className="absolute top-11 left-3 z-50 flex w-fit flex-row items-center gap-1 rounded-full border px-1.5 py-1 shadow-[0_16px_48px_rgba(0,0,0,0.45)] backdrop-blur-2xl select-none"
    >
      {WORKSPACES.map((workspace) => {
        const active = workspace === currentWorkspace;
        return (
          <button
            key={workspace}
            type="button"
            aria-label={`Workspace ${workspace}`}
            aria-current={active ? 'true' : undefined}
            onClick={() => onSelectWorkspace(workspace)}
            style={{
              backgroundColor: active ? theme.palette.accent : theme.palette.surface,
              color: active ? theme.palette.background : theme.palette.textPrimary,
            }}
            className="flex h-6 w-6 items-center justify-center rounded-full text-[11px] font-semibold tabular-nums transition-colors hover:brightness-110"
          >
            {workspace}
          </button>
        );
      })}
    </div>
  );
}
