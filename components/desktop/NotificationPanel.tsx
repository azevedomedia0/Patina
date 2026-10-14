'use client';

import React, { useState } from 'react';
import { DEConfig } from '@/lib/oxide-types';
import { Bell, Download, Image as ImageIcon, Music, X } from 'lucide-react';

interface NotificationPanelProps {
  isOpen: boolean;
  config: DEConfig;
}

type NoticeIcon = 'update' | 'music' | 'image';

interface Notice {
  id: string;
  app: string;
  title: string;
  body: string;
  time: string;
  icon: NoticeIcon;
}

const INITIAL_NOTICES: Notice[] = [
  {
    id: 'update',
    app: 'Software',
    title: 'Oxide 0.9 is ready',
    body: 'Restart to finish installing the update.',
    time: '2m',
    icon: 'update',
  },
  {
    id: 'music',
    app: 'Music',
    title: 'Wayland Drift',
    body: 'Oxide Ensemble · now playing',
    time: '12m',
    icon: 'music',
  },
  {
    id: 'screenshot',
    app: 'Files',
    title: 'Screenshot saved',
    body: 'Pictures/desktop-2026-10-14.png',
    time: '1h',
    icon: 'image',
  },
];

function NoticeIconView({ icon }: { icon: NoticeIcon }) {
  if (icon === 'update') return <Download className="h-4 w-4" />;
  if (icon === 'music') return <Music className="h-4 w-4" />;
  return <ImageIcon className="h-4 w-4" />;
}

export default function NotificationPanel({ isOpen, config }: NotificationPanelProps) {
  const [notices, setNotices] = useState<Notice[]>(INITIAL_NOTICES);

  if (!isOpen) return null;

  const theme = config.activeTheme;

  return (
    <div
      role="dialog"
      aria-label="Notifications"
      style={{
        backgroundColor: theme.palette.panel,
        borderColor: theme.palette.border,
        color: theme.palette.textPrimary,
        fontFamily: theme.fontFamily === 'System Sans' ? 'inherit' : theme.fontFamily,
      }}
      className="absolute top-11 left-1/2 z-50 flex w-[22rem] max-h-[70vh] -translate-x-1/2 flex-col gap-2 overflow-y-auto rounded-[26px] border p-3 shadow-[0_16px_48px_rgba(0,0,0,0.45)] backdrop-blur-2xl select-none"
    >
      <div className="flex items-center justify-between px-1">
        <span className="text-[13px] font-medium">Notifications</span>
        {notices.length > 0 && (
          <button
            type="button"
            onClick={() => setNotices([])}
            className="text-[11px] font-medium transition-opacity hover:opacity-80"
            style={{ color: theme.palette.accent }}
          >
            Clear
          </button>
        )}
      </div>

      {notices.length === 0 ? (
        <div
          className="flex flex-col items-center gap-2 rounded-[18px] px-3 py-8 text-center"
          style={{ backgroundColor: theme.palette.surface, color: theme.palette.textSecondary }}
        >
          <Bell className="h-5 w-5" />
          <span className="text-[13px]">No notifications</span>
        </div>
      ) : (
        notices.map((notice) => (
          <div
            key={notice.id}
            className="flex items-start gap-2.5 rounded-[18px] px-3 py-2.5"
            style={{ backgroundColor: theme.palette.surface }}
          >
            <span
              className="mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-full"
              style={{ backgroundColor: theme.palette.accent, color: theme.palette.background }}
            >
              <NoticeIconView icon={notice.icon} />
            </span>
            <div className="min-w-0 flex-1">
              <div className="flex items-baseline justify-between gap-2">
                <span className="truncate text-[11px]" style={{ color: theme.palette.textSecondary }}>
                  {notice.app}
                </span>
                <span className="shrink-0 text-[11px]" style={{ color: theme.palette.textSecondary }}>
                  {notice.time}
                </span>
              </div>
              <span className="block truncate text-[13px] font-medium leading-tight">{notice.title}</span>
              <span className="block truncate text-[11px] leading-tight" style={{ color: theme.palette.textSecondary }}>
                {notice.body}
              </span>
            </div>
            <button
              type="button"
              aria-label={`Dismiss ${notice.title}`}
              onClick={() => setNotices((current) => current.filter((item) => item.id !== notice.id))}
              className="rounded-full p-1 transition-opacity hover:opacity-80"
              style={{ color: theme.palette.textSecondary }}
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        ))
      )}
    </div>
  );
}
