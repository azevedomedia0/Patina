import type {Metadata} from 'next';
import { Poppins } from 'next/font/google';
import './globals.css';

const poppins = Poppins({
  weight: ['300', '400', '500', '600', '700'],
  subsets: ['latin'],
  display: 'swap',
  variable: '--font-poppins',
});

export const metadata: Metadata = {
  title: 'OxideDE - Modern Lightweight Rust Desktop Environment',
  description: 'Ultra-fast, modern, memory-efficient Linux desktop environment written in Rust with Wayland compositor, dual GPU/software rendering for 32-bit and 64-bit ARM, and full architecture studio.',
  openGraph: {
    title: 'OxideDE - Modern Lightweight Rust Desktop Environment',
    description: 'Ultra-fast, modern, memory-efficient Linux desktop environment written in Rust with Wayland compositor, dual GPU/software rendering for 32-bit and 64-bit ARM, and full architecture studio.',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'OxideDE - Modern Lightweight Rust Desktop Environment',
    description: 'Ultra-fast, modern, memory-efficient Linux desktop environment written in Rust with Wayland compositor, dual GPU/software rendering for 32-bit and 64-bit ARM, and full architecture studio.',
  },
};

export default function RootLayout({children}: {children: React.ReactNode}) {
  return (
    <html lang="en" className={poppins.className}>
      <body className={poppins.className} suppressHydrationWarning>
        {children}
      </body>
    </html>
  );
}
