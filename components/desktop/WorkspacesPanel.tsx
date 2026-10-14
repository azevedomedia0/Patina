'use client';

import React from 'react';
import { DEConfig } from '@/lib/oxide-types';

const WORKSPACES = [
  { id: 1, label: 'Workspace 1' },
  { id: 2, label: 'Workspace 2' },
  { id: 3, label: 'Workspace 3' },
  { id: 4, label: 'Workspace 4' },
] as const;

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
      className="absolute top-11 left-3 z-50 flex w-56 flex-col gap-1 rounded-[26px] border p-3 shadow-[0_16px_48px_rgba(0,0,0,0.45)] backdrop-blur-2xl select-none"
    >
      {WORKSPACES.map((workspace) => {
        const active = workspace.id === currentWorkspace;
        return (
          <button
            key={workspace.id}
            type="button"
            aria-current={active ? 'true' : undefined}
            onClick={() => onSelectWorkspace(workspace.id)}
            style={{
              backgroundColor: active ? theme.palette.accent : theme.palette.surface,
              color: active ? theme.palette.background : theme.palette.textPrimary,
            }}
            className="rounded-[18px] px-3 py-2 text-left text-[13px] font-medium transition-colors hover:brightness-110"
          >
            {workspace.label}
          </button>
        );
      })}
    </div>
  );
}
