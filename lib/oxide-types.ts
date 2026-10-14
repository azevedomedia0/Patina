export type ArchType = 'armv7l' | 'aarch64' | 'x86_64_low' | 'x86_64_high';

export interface HardwareProfile {
  id: ArchType;
  name: string;
  tagline: string;
  architecture: string;
  bits: '32-bit' | '64-bit';
  cpuCores: number;
  cpuModel: string;
  ramTotalMB: number;
  gpuDriver: string;
  compositorBackend: 'pixman / tiny-skia (Software)' | 'OpenGL ES 2.0 / KMS' | 'OpenGL ES 3.1 / Wayland EGL' | 'Vulkan / WGPU';
  idleRamMB: number;
  blurSupported: boolean;
  maxFpsTarget: number;
  defaultLowSpecMode: boolean;
  neonSupported: boolean;
  cacheLineBytes: 32 | 64;
}

export const HARDWARE_PROFILES: Record<ArchType, HardwareProfile> = {
  armv7l: {
    id: 'armv7l',
    name: 'Raspberry Pi 2 / 3 (32-bit)',
    tagline: 'ARMv7-A 32-bit (Cortex-A7 / A53 in 32-bit mode)',
    architecture: 'armv7l (armhf)',
    bits: '32-bit',
    cpuCores: 4,
    cpuModel: 'Broadcom BCM2836/2837 @ 900MHz-1.2GHz',
    ramTotalMB: 1024,
    gpuDriver: 'VideoCore IV (KMS dumb-buffer direct scanout)',
    compositorBackend: 'pixman / tiny-skia (Software)',
    idleRamMB: 28,
    blurSupported: false,
    maxFpsTarget: 60,
    defaultLowSpecMode: true,
    neonSupported: true,
    cacheLineBytes: 32,
  },
  aarch64: {
    id: 'aarch64',
    name: 'Raspberry Pi 4 / 5 (64-bit)',
    tagline: 'ARMv8.2-A 64-bit (Cortex-A72 / A76)',
    architecture: 'aarch64 (arm64)',
    bits: '64-bit',
    cpuCores: 4,
    cpuModel: 'Broadcom BCM2711/2712 @ 1.8GHz-2.4GHz',
    ramTotalMB: 4096,
    gpuDriver: 'Broadcom V3D 4.2 / Mesa 24',
    compositorBackend: 'OpenGL ES 3.1 / Wayland EGL',
    idleRamMB: 42,
    blurSupported: true,
    maxFpsTarget: 60,
    defaultLowSpecMode: false,
    neonSupported: true,
    cacheLineBytes: 64,
  },
  x86_64_low: {
    id: 'x86_64_low',
    name: 'Low-Spec Netbook / Mini PC',
    tagline: 'x86_64 Low-Power Celeron / Pentium / Atom',
    architecture: 'x86_64',
    bits: '64-bit',
    cpuCores: 2,
    cpuModel: 'Intel Celeron N3350 / N4020 @ 1.10GHz',
    ramTotalMB: 2048,
    gpuDriver: 'Intel HD Graphics 500 (i915 DRM/KMS)',
    compositorBackend: 'OpenGL ES 2.0 / KMS',
    idleRamMB: 36,
    blurSupported: false,
    maxFpsTarget: 60,
    defaultLowSpecMode: true,
    neonSupported: false,
    cacheLineBytes: 64,
  },
  x86_64_high: {
    id: 'x86_64_high',
    name: 'Modern Workstation',
    tagline: 'x86_64 AMD Ryzen / Intel Core',
    architecture: 'x86_64',
    bits: '64-bit',
    cpuCores: 8,
    cpuModel: 'AMD Ryzen 7 7840U / Intel Core i7',
    ramTotalMB: 16384,
    gpuDriver: 'AMD Radeon Graphics (radeonsi/RADV)',
    compositorBackend: 'Vulkan / WGPU',
    idleRamMB: 48,
    blurSupported: true,
    maxFpsTarget: 144,
    defaultLowSpecMode: false,
    neonSupported: false,
    cacheLineBytes: 64,
  },
};

export type AppId =
  | 'terminal'
  | 'sysmon'
  | 'theme-studio'
  | 'profiler'
  | 'files'
  | 'codebase'
  | 'benchmark'
  | 'settings'
  | 'guide';

export interface AppDefinition {
  id: AppId;
  title: string;
  iconName: string;
  category: 'System' | 'Development' | 'Utilities' | 'Documentation';
  description: string;
  defaultWidth: number;
  defaultHeight: number;
}

export interface WindowState {
  id: string;
  appId: AppId;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  isMinimized: boolean;
  isMaximized: boolean;
  zIndex: number;
  workspace: number;
}

// -----------------------------------------------------------------------------
// DYNAMIC THEMING ENGINE DEFINITIONS
// -----------------------------------------------------------------------------

export type FontFamilyType = 'Poppins' | 'JetBrains Mono' | 'Plus Jakarta Sans' | 'DM Sans' | 'Fira Code' | 'System Sans';
export type IconThemeType = 'oxide-minimal' | 'lucid-wire' | 'neo-glyph' | 'monochrome-lowres';

export interface ColorPalette {
  background: string;
  surface: string;
  panel: string;
  border: string;
  textPrimary: string;
  textSecondary: string;
  accent: string;
  accentHover: string;
  success: string;
  warning: string;
  danger: string;
}

export interface WindowStyleConfig {
  borderRadius: 0 | 4 | 8 | 12 | 16 | 24; // 0 = sharp direct-scanout for armv7l, 24 = pill rounded
  borderWidth: 1 | 2;
  backdropBlur: boolean;
  windowControls: 'right' | 'left';
  compactTitlebar: boolean;
  isPillShape?: boolean;
  isMacStyle?: boolean;
}

export interface DynamicTheme {
  id: string;
  name: string;
  description: string;
  palette: ColorPalette;
  fontFamily: FontFamilyType;
  fontSize: 'sm' | 'base' | 'lg';
  iconTheme: IconThemeType;
  windowStyle: WindowStyleConfig;
  isCustom?: boolean;
}

export const PRESET_THEMES: Record<string, DynamicTheme> = {
  'pill-fluid': {
    id: 'pill-fluid',
    name: 'Pill Capsule Fluid',
    description: 'Everything pill-shaped and ultra-rounded: smooth capsule corners, rounded-full controls, and fluid surfaces',
    palette: {
      background: '#0b0d14',
      surface: '#151824',
      panel: '#10121c',
      border: '#2e3347',
      textPrimary: '#f8fafc',
      textSecondary: '#94a3b8',
      accent: '#a855f7',
      accentHover: '#9333ea',
      success: '#10b981',
      warning: '#f59e0b',
      danger: '#ef4444',
    },
    fontFamily: 'Poppins',
    fontSize: 'base',
    iconTheme: 'neo-glyph',
    windowStyle: {
      borderRadius: 24,
      borderWidth: 1,
      backdropBlur: true,
      windowControls: 'right',
      compactTitlebar: false,
      isPillShape: true,
      isMacStyle: false,
    },
  },
  'macos-cupertino': {
    id: 'macos-cupertino',
    name: 'macOS Cupertino Glass',
    description: 'macOS-inspired frosted glass: left traffic lights, centered titles, and Apple Silicon blue accents',
    palette: {
      background: '#16161a',
      surface: '#202026',
      panel: '#1a1a20',
      border: '#33333e',
      textPrimary: '#f5f5f7',
      textSecondary: '#a1a1a6',
      accent: '#007aff',
      accentHover: '#0062cc',
      success: '#34c759',
      warning: '#ff9f0a',
      danger: '#ff3b30',
    },
    fontFamily: 'Poppins',
    fontSize: 'base',
    iconTheme: 'lucid-wire',
    windowStyle: {
      borderRadius: 16,
      borderWidth: 1,
      backdropBlur: true,
      windowControls: 'left',
      compactTitlebar: false,
      isPillShape: false,
      isMacStyle: true,
    },
  },
  'oxide-dark': {
    id: 'oxide-dark',
    name: 'Oxide Obsidian Core',
    description: 'Signature dark obsidian with warm anodized amber copper accents',
    palette: {
      background: '#090a0f',
      surface: '#11131a',
      panel: '#0d0f14',
      border: '#272a38',
      textPrimary: '#f8fafc',
      textSecondary: '#94a3b8',
      accent: '#f59e0b',
      accentHover: '#d97706',
      success: '#10b981',
      warning: '#f59e0b',
      danger: '#ef4444',
    },
    fontFamily: 'Poppins',
    fontSize: 'base',
    iconTheme: 'oxide-minimal',
    windowStyle: {
      borderRadius: 8,
      borderWidth: 1,
      backdropBlur: true,
      windowControls: 'right',
      compactTitlebar: false,
      isPillShape: false,
      isMacStyle: false,
    },
  },
  'tokyo-night': {
    id: 'tokyo-night',
    name: 'Tokyo Night Wave',
    description: 'Deep navy cyberpunk tones with neon cyan and lavender accents',
    palette: {
      background: '#1a1b26',
      surface: '#24283b',
      panel: '#16161e',
      border: '#414868',
      textPrimary: '#c0caf5',
      textSecondary: '#7aa2f7',
      accent: '#7aa2f7',
      accentHover: '#bb9af7',
      success: '#9ece6a',
      warning: '#e0af68',
      danger: '#f7768e',
    },
    fontFamily: 'Plus Jakarta Sans',
    fontSize: 'base',
    iconTheme: 'neo-glyph',
    windowStyle: {
      borderRadius: 12,
      borderWidth: 1,
      backdropBlur: true,
      windowControls: 'right',
      compactTitlebar: false,
      isPillShape: false,
      isMacStyle: false,
    },
  },
  'nordic-frost': {
    id: 'nordic-frost',
    name: 'Nordic Arctic Frost',
    description: 'Crisp glacial blues with high-readability polar typography',
    palette: {
      background: '#0e141b',
      surface: '#17202a',
      panel: '#0b1016',
      border: '#2c3b4d',
      textPrimary: '#eceff4',
      textSecondary: '#88c0d0',
      accent: '#38bdf8',
      accentHover: '#0284c7',
      success: '#a3be8c',
      warning: '#ebcb8b',
      danger: '#bf616a',
    },
    fontFamily: 'DM Sans',
    fontSize: 'base',
    iconTheme: 'lucid-wire',
    windowStyle: {
      borderRadius: 8,
      borderWidth: 1,
      backdropBlur: true,
      windowControls: 'right',
      compactTitlebar: false,
      isPillShape: false,
      isMacStyle: false,
    },
  },
  'cosmic-desktop': {
    id: 'cosmic-desktop',
    name: 'COSMIC Desktop',
    description: 'System76 COSMIC-inspired charcoal surfaces, softly rounded windows, and a blue accent on a blurred panel',
    palette: {
      background: '#1c1c1c',
      surface: '#2b2b2b',
      panel: '#242424',
      border: '#454545',
      textPrimary: '#f2f2f2',
      textSecondary: '#c0c0c0',
      accent: '#4482ff',
      accentHover: '#6b9bff',
      success: '#2ec27e',
      warning: '#e5a50a',
      danger: '#ed5c5c',
    },
    fontFamily: 'System Sans',
    fontSize: 'base',
    iconTheme: 'lucid-wire',
    windowStyle: {
      borderRadius: 12,
      borderWidth: 1,
      backdropBlur: true,
      windowControls: 'right',
      compactTitlebar: false,
      isPillShape: false,
      isMacStyle: false,
    },
  },
  'monochrome-lowres': {
    id: 'monochrome-lowres',
    name: 'OLED Zero-Cost (Ultra Low-Spec)',
    description: 'True #000000 black canvas, 0px border radius, 0% blur overhead for 32-bit ARM',
    palette: {
      background: '#000000',
      surface: '#0d0d0d',
      panel: '#050505',
      border: '#262626',
      textPrimary: '#ffffff',
      textSecondary: '#a3a3a3',
      accent: '#10b981',
      accentHover: '#059669',
      success: '#10b981',
      warning: '#eab308',
      danger: '#ef4444',
    },
    fontFamily: 'JetBrains Mono',
    fontSize: 'sm',
    iconTheme: 'monochrome-lowres',
    windowStyle: {
      borderRadius: 0,
      borderWidth: 1,
      backdropBlur: false,
      windowControls: 'right',
      compactTitlebar: true,
      isPillShape: false,
      isMacStyle: false,
    },
  },
};

// -----------------------------------------------------------------------------
// PERFORMANCE PROFILING & OPTIMIZATION DEFINITIONS
// -----------------------------------------------------------------------------

export interface ProfilerTelemetry {
  frameTimeUs: number; // microseconds
  frameBudgetUs: number; // 16666 µs for 60Hz
  cpuLoadPct: number;
  activeRamMB: number;
  damageRectsPerFrame: number;
  neonVectorPasses: number;
  bufferPoolAllocations: number;
  hardwarePlanesActive: number;
  l1CacheHitRatePct: number;
}

export interface OptimizationSettings {
  neonSimd: boolean; // ARM NEON SIMD vectorization (armv7l & aarch64)
  bumpAllocator: boolean; // Zero-heap-allocation per frame
  kmsHardwarePlanes: boolean; // Offload cursor & panel to DRM hardware planes
  cacheAlignment64: boolean; // 64-byte L1 cache-line data alignment
  directScanout: boolean; // Bypass blending for fullscreen windows
  damageTracking: boolean; // Smithay dirty rects
  ltoAggressive: boolean; // Fat LTO + 1 codegen unit
}

export interface DEConfig {
  activeTheme: DynamicTheme;
  wallpaper: 'stock' | 'oxide-mesh' | 'nordic-aurora' | 'rust-metal' | 'minimal-charcoal';
  tilingMode: boolean; // false = floating, true = dynamic auto-tiling
  lowSpecMode: boolean; // bypass blur, reduce shadow radius, direct scanout
  fontScale: 90 | 100 | 110;
  damageTracking: boolean; // re-render only damaged rects
  showFpsOverlay: boolean;
  animationsEnabled: boolean;
  optimizations: OptimizationSettings;
}
