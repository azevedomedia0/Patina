'use client';

import React from 'react';
import DesktopEnvironment from '@/components/desktop/DesktopEnvironment';

export default function HomePage() {
  return (
    <div className="w-full h-screen overflow-hidden bg-neutral-950 text-neutral-100 flex flex-col">
      <DesktopEnvironment />
    </div>
  );
}
