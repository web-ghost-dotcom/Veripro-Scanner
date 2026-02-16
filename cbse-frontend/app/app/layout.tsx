'use client';

import { createContext, useContext, ReactNode, useState, useEffect } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { ConnectButton } from '@rainbow-me/rainbowkit';
import { useAccount } from 'wagmi';
import { WalletProvider } from '../lib/providers';

interface AuthContextType {
    isConnected: boolean;
    address: string | undefined;
}

const AuthContext = createContext<AuthContextType>({
    isConnected: false,
    address: undefined,
});

export const useAuth = () => useContext(AuthContext);

function Sidebar() {
    const pathname = usePathname();

    const navItems = [
        { href: '/app', label: 'Dashboard', id: 'dashboard' },
        { href: '/app/projects', label: 'Projects', id: 'projects' },
        { href: '/app/verify', label: 'Verify', id: 'verify' },
        { href: '/app/results', label: 'Results', id: 'results' },
        { href: '/app/settings', label: 'Settings', id: 'settings' },
    ];

    return (
        <aside className="fixed left-0 top-0 bottom-0 w-64 bg-zinc-950 border-r border-zinc-900 flex flex-col z-50">
            <div className="p-6 border-b border-zinc-900">
                <Link href="/" className="text-xl font-light tracking-tight text-white">
                    VERIPRO
                </Link>
            </div>

            <nav className="flex-1 p-4">
                <Link
                    href="/app/projects/new"
                    className="flex items-center justify-center w-full px-4 py-2 mb-6 text-sm font-medium bg-white rounded hover:bg-zinc-200 transition-colors"
                    style={{ color: '#000' }}
                >
                    <span className="mr-2 text-lg leading-none">+</span> New Project
                </Link>

                <ul className="space-y-1">
                    {navItems.map((item) => {
                        const isActive = pathname === item.href ||
                            (item.href !== '/app' && pathname.startsWith(item.href));
                        return (
                            <li key={item.id}>
                                <Link
                                    href={item.href}
                                    className={`block px-4 py-3 rounded text-sm transition-colors ${isActive
                                        ? 'bg-zinc-900 text-white'
                                        : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/50'
                                        }`}
                                >
                                    {item.label}
                                </Link>
                            </li>
                        );
                    })}
                </ul>
            </nav>

            <div className="p-4 border-t border-zinc-900">
                <Link
                    href="/docs"
                    className="block px-4 py-2 text-sm text-zinc-500 hover:text-zinc-300 transition-colors"
                >
                    Documentation
                </Link>
                <Link
                    href="/"
                    className="block px-4 py-2 text-sm text-zinc-500 hover:text-zinc-300 transition-colors"
                >
                    Back to Home
                </Link>
            </div>
        </aside>
    );
}

function TopBar() {
    return (
        <header className="fixed top-0 left-64 right-0 h-16 bg-black border-b border-zinc-900 flex items-center justify-between px-6 z-40">
            <div className="text-sm text-zinc-500">
                Workspace
            </div>

            <div className="flex items-center gap-4">
                <ConnectButton
                    chainStatus="icon"
                    showBalance={false}
                    accountStatus={{
                        smallScreen: 'avatar',
                        largeScreen: 'full',
                    }}
                />
            </div>
        </header>
    );
}

function AppContent({ children }: { children: ReactNode }) {
    const { address, isConnected } = useAccount();
    const [mounted, setMounted] = useState(false);

    useEffect(() => {
        // eslint-disable-next-line react-hooks/set-state-in-effect
        setMounted(true);
    }, []);

    if (!mounted) {
        return (
            <div className="min-h-screen bg-black text-white">
                {/* Minimal loading state */}
            </div>
        );
    }

    return (
        <AuthContext.Provider value={{ isConnected, address }}>
            <div className="min-h-screen bg-black text-white">
                <Sidebar />
                <TopBar />
                <main className="ml-64 pt-16 min-h-screen">
                    {children}
                </main>
            </div>
        </AuthContext.Provider>
    );
}

export default function AppLayout({ children }: { children: ReactNode }) {
    return (
        <WalletProvider>
            <AppContent>{children}</AppContent>
        </WalletProvider>
    );
}
