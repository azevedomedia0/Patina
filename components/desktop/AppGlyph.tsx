import Image from 'next/image';
import { AppId } from '@/lib/oxide-types';

const PAPIRUS_ICONS: Record<AppId, string> = {
  terminal: '/icons/papirus/terminal.svg',
  'theme-studio': '/icons/papirus/theme-studio.svg',
  profiler: '/icons/papirus/profiler.svg',
  sysmon: '/icons/papirus/sysmon.svg',
  codebase: '/icons/papirus/codebase.svg',
  benchmark: '/icons/papirus/benchmark.svg',
  files: '/icons/papirus/files.svg',
  settings: '/icons/papirus/settings.svg',
  guide: '/icons/papirus/guide.svg',
};

export default function AppGlyph({
  id,
  className = 'h-8 w-8',
  size,
}: {
  id: AppId;
  className?: string;
  size?: number;
}) {
  return (
    <Image
      src={PAPIRUS_ICONS[id]}
      alt=""
      width={48}
      height={48}
      unoptimized
      className={size ? 'object-contain' : `${className} object-contain`}
      style={size ? { width: size, height: size } : undefined}
    />
  );
}
