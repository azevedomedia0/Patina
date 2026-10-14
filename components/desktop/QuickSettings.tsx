'use client';

import React, { useRef, useState } from 'react';
import { DEConfig, AppId } from '@/lib/oxide-types';
import { Volume2, VolumeX, Sun, SunDim, Cast, Wifi, WifiOff, Bluetooth, BluetoothOff, Shield, ShieldOff, Play, Pause, SkipBack, SkipForward, Shuffle, Repeat, Repeat1, Music, LogOut, RotateCcw, Power } from 'lucide-react';

export type SessionAction = 'log-off' | 'restart' | 'shutdown';

interface QuickSettingsProps {
  isOpen: boolean;
  onClose: () => void;
  config: DEConfig;
  onUpdateConfig: (partial: Partial<DEConfig>) => void;
  onOpenApp: (appId: AppId) => void;
  onSessionAction: (action: SessionAction) => void;
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
  shuffle,
  repeat,
  onTogglePlay,
  onToggleShuffle,
  onToggleRepeat,
  onPrevious,
  onNext,
}: {
  idleBg: string;
  idleColor: string;
  accent: string;
  activeColor: string;
  trackIndex: number;
  playing: boolean;
  shuffle: boolean;
  repeat: boolean;
  onTogglePlay: () => void;
  onToggleShuffle: () => void;
  onToggleRepeat: () => void;
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
      <div>
        <div className="mb-1 flex items-center justify-center gap-3">
          <button
            type="button"
            aria-label="Shuffle"
            aria-pressed={shuffle}
            onClick={onToggleShuffle}
            style={{ color: shuffle ? accent : undefined }}
            className="rounded-full p-1 hover:bg-white/10"
          >
            <Shuffle className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            aria-label="Repeat"
            aria-pressed={repeat}
            onClick={onToggleRepeat}
            style={{ color: repeat ? accent : undefined }}
            className="rounded-full p-1 hover:bg-white/10"
          >
            {repeat ? <Repeat1 className="h-3.5 w-3.5" /> : <Repeat className="h-3.5 w-3.5" />}
          </button>
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
  onSessionAction,
  brightness,
  onBrightnessChange,
}: QuickSettingsProps) {
  const [volume, setVolume] = useState(75);
  const lastVolume = useRef(75);
  const [casting, setCasting] = useState(false);
  const [wifiOn, setWifiOn] = useState(true);
  const [bluetoothOn, setBluetoothOn] = useState(true);
  const [vpnOn, setVpnOn] = useState(false);
  const [trackIndex, setTrackIndex] = useState(0);
  const [playing, setPlaying] = useState(true);
  const [shuffle, setShuffle] = useState(false);
  const [repeat, setRepeat] = useState(false);

  if (!isOpen) return null;

  const toggleMute = () => {
    if (volume === 0) {
      setVolume(lastVolume.current > 0 ? lastVolume.current : 75);
      return;
    }
    lastVolume.current = volume;
    setVolume(0);
  };

  const theme = config.activeTheme;
  const accent = theme.palette.accent;
  const idleBg = theme.palette.surface;
  const idleColor = theme.palette.textPrimary;
  const activeColor = theme.palette.background;
  const openApp = (appId: AppId) => {
    onClose();
    onOpenApp(appId);
  };
  const runSession = (action: SessionAction) => {
    onClose();
    onSessionAction(action);
  };
  const sessionButtonClass =
    'flex min-w-0 flex-1 items-center justify-center rounded-[18px] px-1.5 py-2.5 text-[13px] font-medium transition-colors hover:brightness-110';

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
          shuffle={shuffle}
          repeat={repeat}
          onTogglePlay={() => setPlaying((on) => !on)}
          onToggleShuffle={() => setShuffle((on) => !on)}
          onToggleRepeat={() => setRepeat((on) => !on)}
          onPrevious={() =>
            setTrackIndex((index) => {
              if (repeat) return index;
              if (shuffle) {
                let next = index;
                while (next === index) next = Math.floor(Math.random() * NOW_PLAYING.length);
                return next;
              }
              return (index + NOW_PLAYING.length - 1) % NOW_PLAYING.length;
            })
          }
          onNext={() =>
            setTrackIndex((index) => {
              if (repeat) return index;
              if (shuffle) {
                let next = index;
                while (next === index) next = Math.floor(Math.random() * NOW_PLAYING.length);
                return next;
              }
              return (index + 1) % NOW_PLAYING.length;
            })
          }
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
          active={vpnOn}
          icon={vpnOn ? <Shield className="h-5 w-5" /> : <ShieldOff className="h-5 w-5" />}
          title="VPN"
          subtitle={vpnOn ? 'Connected' : 'Off'}
          onClick={() => setVpnOn((on) => !on)}
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
        <button
          type="button"
          aria-label={volume === 0 ? 'Unmute' : 'Mute'}
          aria-pressed={volume === 0}
          onClick={toggleMute}
          className="shrink-0 rounded-full p-0.5 transition-opacity hover:opacity-80"
        >
          {volume === 0 ? <VolumeX className="h-5 w-5" /> : <Volume2 className="h-5 w-5" />}
        </button>
        <input
          type="range"
          min={0}
          max={100}
          value={volume}
          aria-label="Volume"
          onChange={(event) => {
            const next = Number(event.target.value);
            if (next > 0) lastVolume.current = next;
            setVolume(next);
          }}
          style={{
            background: `linear-gradient(to right, ${accent} ${volume}%, rgba(255,255,255,0.18) ${volume}%)`,
          }}
          className="h-1 w-full cursor-pointer appearance-none rounded-full [&::-webkit-slider-thumb]:h-3.5 [&::-webkit-slider-thumb]:w-3.5 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white [&::-moz-range-thumb]:h-3.5 [&::-moz-range-thumb]:w-3.5 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:bg-white"
        />
      </div>

      <div className="flex gap-2">
        <button
          type="button"
          onClick={() => openApp('settings')}
          style={{ backgroundColor: idleBg, color: idleColor }}
          className="w-1/2 shrink-0 rounded-[18px] py-2.5 text-[13px] font-medium transition-colors hover:brightness-110"
        >
          All Settings
        </button>
        <button
          type="button"
          aria-label="Log Off"
          title="Log Off"
          onClick={() => runSession('log-off')}
          style={{ backgroundColor: idleBg, color: idleColor }}
          className={sessionButtonClass}
        >
          <LogOut className="h-4 w-4" aria-hidden="true" />
        </button>
        <button
          type="button"
          aria-label="Restart"
          title="Restart"
          onClick={() => runSession('restart')}
          style={{ backgroundColor: idleBg, color: idleColor }}
          className={sessionButtonClass}
        >
          <RotateCcw className="h-4 w-4" aria-hidden="true" />
        </button>
        <button
          type="button"
          aria-label="Shutdown"
          title="Shutdown"
          onClick={() => runSession('shutdown')}
          style={{ backgroundColor: idleBg, color: idleColor }}
          className={sessionButtonClass}
        >
          <Power className="h-4 w-4" aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}
