'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';

export default function Navbar() {
    const pathname = usePathname();

    // Hide navbar on app routes (editor platform has its own layout)
    if (pathname?.startsWith('/app')) return null;

    const isActive = (path: string) => pathname === path;

    return (
        <header className="fixed top-0 left-0 right-0 z-50 bg-black border-b border-zinc-900">
            <div className="max-w-7xl mx-auto px-6 py-4 flex items-center justify-between">
                <Link href="/" className="text-xl font-light tracking-tight text-white hover:text-zinc-400 transition-colors">
                    VERIPRO
                </Link>
                <nav className="flex items-center gap-8 text-sm">
                    <Link
                        href="/"
                        className={`transition-colors ${isActive('/') ? 'text-white' : 'text-zinc-500 hover:text-white'}`}
                    >
                        Home
                    </Link>
                    <Link
                        href="/demo"
                        className={`transition-colors ${isActive('/demo') ? 'text-white' : 'text-zinc-500 hover:text-white'}`}
                    >
                        Demo
                    </Link>
                    <Link
                        href="/docs"
                        className={`transition-colors ${isActive('/docs') ? 'text-white' : 'text-zinc-500 hover:text-white'}`}
                    >
                        Docs
                    </Link>
                    <Link
                        href="/app"
                        className="px-4 py-2 border border-zinc-800 rounded-sm text-zinc-400 hover:text-white hover:border-zinc-700 transition-colors"
                    >
                        Open Platform
                    </Link>
                </nav>
            </div>
        </header>
    );
}
