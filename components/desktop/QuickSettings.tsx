'use client';

import React, { useState } from 'react';
import { DEConfig, AppId } from '@/lib/oxide-types';
import { Palette, Volume2, VolumeX, Sun, SunDim, Cast, Wifi, WifiOff, Bluetooth, BluetoothOff, Play, Pause, SkipBack, SkipForward, Music } from 'lucide-react';

interface QuickSettingsProps {
  isOpen: boolean;
  onClose: () => void;
  config: DEConfig;
  onUpdateConfig: (partial: Partial<DEConfig>) => void;
  onOpenApp: (appId: AppId) => void;
  brightness: number;
  onBrightnessChange: (value: number) => void;
}

const NOW_PLAYING = [
  { title: 'Wayland Drift', artist: 'Oxide Ensemble' },
  { title: 'Charcoal Hour', artist: 'Seatd Quartet' },
  { title: 'Scanout', artist: 'KMS Trio' },
];

function NowPlaying({
  idleBg,
  idleColor,
  accent,
  activeColor,
  trackIndex,
  playing,
  onTogglePlay,
  onPrevious,
  onNext,
}: {
  idleBg: string;
  idleColor: string;
  accent: string;
  activeColor: string;
  trackIndex: number;
  playing: boolean;
  onTogglePlay: () => void;
  onPrevious: () => void;
  onNext: () => void;
}) {
  const track = NOW_PLAYING[trackIndex];
  return (
    <div
      aria-label="Now playing"
      style={{ backgroundColor: idleBg, color: idleColor }}
      className="row-span-2 flex min-h-[132px] flex-col justify-between rounded-[18px] px-3 py-2.5"
    >
      <div className="flex items-center gap-2.5">
        <div
          className="flex h-11 w-11 shrink-0 items-center justify-center rounded-[12px]"
          style={{ backgroundColor: accent, color: activeColor }}
        >
          <Music className="h-5 w-5" />
        </div>
        <div className="min-w-0">
          <span className="block truncate text-[11px] leading-tight opacity-80">{playing ? 'Now playing' : 'Paused'}</span>
          <span className="block truncate text-[13px] font-medium leading-tight">{track.title}</span>
          <span className="block truncate text-[11px] leading-tight opacity-80">{track.artist}</span>
        </div>
      </div>
      <div className="flex items-center justify-center gap-2">
        <button type="button" aria-label="Previous" onClick={onPrevious} className="rounded-full p-1.5 hover:bg-white/10">
          <SkipBack className="h-4 w-4" />
        </button>
        <button
          type="button"
          aria-label={playing ? 'Pause' : 'Play'}
          aria-pressed={playing}
          onClick={onTogglePlay}
          style={{ backgroundColor: accent, color: activeColor }}
          className="rounded-full p-2"
        >
          {playing ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
        </button>
        <button type="button" aria-label="Next" onClick={onNext} className="rounded-full p-1.5 hover:bg-white/10">
          <SkipForward className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}

function QuickToggle({
  active,
  icon,
  title,
  subtitle,
  onClick,
  accent,
  idleBg,
  idleColor,
  activeColor,
}: {
  active?: boolean;
  icon: React.ReactNode;
  title: string;
  subtitle: string;
  onClick: () => void;
  accent: string;
  idleBg: string;
  idleColor: string;
  activeColor: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      style={{
        backgroundColor: active ? accent : idleBg,
        color: active ? activeColor : idleColor,
      }}
      className="flex min-h-[62px] items-center gap-2.5 rounded-[18px] px-3 py-2 text-left transition-colors hover:brightness-110"
    >
      <span className="shrink-0">{icon}</span>
      <span className="min-w-0">
        <span className="block truncate text-[13px] font-medium leading-tight">{title}</span>
        <span className="block truncate text-[11px] leading-tight opacity-80">{subtitle}</span>
      </span>
    </button>
  );
}

export default function QuickSettings({
  isOpen,
  onClose,
  config,
  onUpdateConfig,
  onOpenApp,
  brightness,
  onBrightnessChange,
}: QuickSettingsProps) {
  const [volume, setVolume] = useState(75);
  const [casting, setCasting] = useState(false);
  const [wifiOn, setWifiOn] = useState(true);
  const [bluetoothOn, setBluetoothOn] = useState(true);
  const [trackIndex, setTrackIndex] = useState(0);
  const [playing, setPlaying] = useState(true);

  if (!isOpen) return null;

  const theme = config.activeTheme;
  const accent = theme.palette.accent;
  const idleBg = theme.palette.surface;
  const idleColor = theme.palette.textPrimary;
  const activeColor = theme.palette.background;
  const openApp = (appId: AppId) => {
    onClose();
    onOpenApp(appId);
  };

  return (
    <div
      style={{
        backgroundColor: theme.palette.panel,
        borderColor: theme.palette.border,
        color: theme.palette.textPrimary,
        fontFamily: theme.fontFamily === 'System Sans' ? 'inherit' : theme.fontFamily,
      }}
      className="absolute top-11 right-2 z-50 flex w-[23.5rem] max-h-[88vh] flex-col gap-2 overflow-y-auto rounded-[26px] border p-3 shadow-[0_16px_48px_rgba(0,0,0,0.45)] backdrop-blur-2xl select-none"
    >
      <div className="grid grid-cols-2 gap-2">
        <QuickToggle
          active={wifiOn}
          icon={wifiOn ? <Wifi className="h-5 w-5" /> : <WifiOff className="h-5 w-5" />}
          title="Wi-Fi"
          subtitle={wifiOn ? 'Connected' : 'Off'}
          onClick={() => setWifiOn((on) => !on)}
          accent={accent}
          idleBg={idleBg}
          idleColor={idleColor}
          activeColor={activeColor}
        />
        <NowPlaying
          idleBg={idleBg}
          idleColor={idleColor}
          accent={accent}
          activeColor={activeColor}
          trackIndex={trackIndex}
          playing={playing}
          onTogglePlay={() => setPlaying((on) => !on)}
          onPrevious={() => setTrackIndex((index) => (index + NOW_PLAYING.length - 1) % NOW_PLAYING.length)}
          onNext={() => setTrackIndex((index) => (index + 1) % NOW_PLAYING.length)}
        />
        <QuickToggle
          active={bluetoothOn}
          icon={bluetoothOn ? <Bluetooth className="h-5 w-5" /> : <BluetoothOff className="h-5 w-5" />}
          title="Bluetooth"
          subtitle={bluetoothOn ? 'On' : 'Off'}
          onClick={() => setBluetoothOn((on) => !on)}
          accent={accent}
          idleBg={idleBg}
          idleColor={idleColor}
          activeColor={activeColor}
        />
        <QuickToggle
          icon={<Palette className="h-5 w-5" />}
          title="Themes"
          subtitle="Studio"
          onClick={() => openApp('theme-studio')}
          accent={accent}
          idleBg={idleBg}
          idleColor={idleColor}
          activeColor={activeColor}
        />
        <QuickToggle
          active={casting}
          icon={<Cast className="h-5 w-5" />}
          title="Casting"
          subtitle={casting ? 'On' : 'Off'}
          onClick={() => setCasting((on) => !on)}
          accent={accent}
          idleBg={idleBg}
          idleColor={idleColor}
          activeColor={activeColor}
        />
      </div>

      <div
        className="flex items-center gap-3 rounded-[18px] px-3 py-2.5"
        style={{ backgroundColor: idleBg, color: idleColor }}
      >
        {brightness === 0 ? <SunDim className="h-5 w-5 shrink-0" /> : <Sun className="h-5 w-5 shrink-0" />}
        <input
          type="range"
          min={0}
          max={100}
          value={brightness}
          aria-label="Brightness"
          onChange={(event) => onBrightnessChange(Number(event.target.value))}
          style={{
            background: `linear-gradient(to right, ${accent} ${brightness}%, rgba(255,255,255,0.18) ${brightness}%)`,
          }}
          className="h-1 w-full cursor-pointer appearance-none rounded-full [&::-webkit-slider-thumb]:h-3.5 [&::-webkit-slider-thumb]:w-3.5 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white [&::-moz-range-thumb]:h-3.5 [&::-moz-range-thumb]:w-3.5 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:bg-white"
        />
      </div>

      <div
        className="flex items-center gap-3 rounded-[18px] px-3 py-2.5"
        style={{ backgroundColor: idleBg, color: idleColor }}
      >
        {volume === 0 ? <VolumeX className="h-5 w-5 shrink-0" /> : <Volume2 className="h-5 w-5 shrink-0" />}
        <input
          type="range"
          min={0}
          max={100}
          value={volume}
          aria-label="Volume"
          onChange={(event) => setVolume(Number(event.target.value))}
          style={{
            background: `linear-gradient(to right, ${accent} ${volume}%, rgba(255,255,255,0.18) ${volume}%)`,
          }}
          className="h-1 w-full cursor-pointer appearance-none rounded-full [&::-webkit-slider-thumb]:h-3.5 [&::-webkit-slider-thumb]:w-3.5 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white [&::-moz-range-thumb]:h-3.5 [&::-moz-range-thumb]:w-3.5 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:bg-white"
        />
      </div>

      <button
        type="button"
        onClick={() => openApp('settings')}
        style={{ backgroundColor: idleBg, color: idleColor }}
        className="rounded-[18px] py-2.5 text-[13px] font-medium transition-colors hover:brightness-110"
      >
        All Settings
      </button>
    </div>
  );
}
