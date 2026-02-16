'use client';

import { useState, useEffect } from 'react';
import { useSearchParams } from 'next/navigation';
import { useAuth } from '../layout';
import { getSettings, updateSettings, type UserSettings } from '../../lib/store';
import { ConnectButton } from '@rainbow-me/rainbowkit';
import { useDisconnect } from 'wagmi';

interface SettingsSection {
    id: string;
    title: string;
    description: string;
}

const SECTIONS: SettingsSection[] = [
    { id: 'account', title: 'Account', description: 'Manage your wallet connection and profile' },
    { id: 'github', title: 'GitHub Integration', description: 'Connect and manage GitHub repositories' },
    { id: 'verification', title: 'Verification', description: 'Configure verification preferences' },
    { id: 'notifications', title: 'Notifications', description: 'Email and in-app notification settings' },
];

// Helper to read cookies on client side
function getCookie(name: string): string | null {
    if (typeof document === 'undefined') return null;
    const match = document.cookie.match(new RegExp('(^| )' + name + '=([^;]+)'));
    return match ? match[2] : null;
}

function deleteCookie(name: string) {
    if (typeof document === 'undefined') return;
    document.cookie = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;`;
}

export default function SettingsPage() {
    const { isConnected, address } = useAuth();
    const { disconnect } = useDisconnect();
    const searchParams = useSearchParams();
    const [activeSection, setActiveSection] = useState('account');
    const [settings, setSettings] = useState<UserSettings | null>(null);
    const [saved, setSaved] = useState(false);
    const [githubUsername, setGithubUsername] = useState<string | null>(null);
    const [githubMessage, setGithubMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

    // Check for GitHub OAuth result from URL params
    useEffect(() => {
        const success = searchParams.get('success');
        const error = searchParams.get('error');

        if (success === 'github_connected') {
            // eslint-disable-next-line react-hooks/set-state-in-effect
            setGithubMessage({ type: 'success', text: 'GitHub account connected successfully!' });
            setActiveSection('github');
            // Clear URL params
            window.history.replaceState({}, '', '/app/settings');
        } else if (error) {
            const message = searchParams.get('message') || 'Failed to connect GitHub account';
            // eslint-disable-next-line react-hooks/set-state-in-effect
            setGithubMessage({ type: 'error', text: message });
            setActiveSection('github');
            window.history.replaceState({}, '', '/app/settings');
        }
    }, [searchParams]);

    // Check for GitHub username cookie
    useEffect(() => {
        const username = getCookie('github_username');
        if (username) {
            // eslint-disable-next-line react-hooks/set-state-in-effect
            setGithubUsername(username);
        }
    }, [githubMessage]);

    const handleGitHubConnect = () => {
        window.location.href = '/api/github/auth';
    };

    const handleGitHubDisconnect = () => {
        deleteCookie('github_access_token');
        deleteCookie('github_username');
        setGithubUsername(null);
        setGithubMessage({ type: 'success', text: 'GitHub account disconnected' });
    };

    useEffect(() => {
        if (isConnected) {
            // eslint-disable-next-line react-hooks/set-state-in-effect
            setSettings(getSettings());
        }
    }, [isConnected]);

    const handleSave = () => {
        if (settings) {
            updateSettings(settings);
            setSaved(true);
            setTimeout(() => setSaved(false), 2000);
        }
    };

    if (!isConnected) {
        return (
            <div className="flex items-center justify-center min-h-[calc(100vh-64px)]">
                <div className="text-center max-w-md">
                    <h1 className="text-3xl font-light mb-4">Settings</h1>
                    <p className="text-zinc-500 mb-8">
                        Connect your wallet to access settings.
                    </p>
                    <div className="flex justify-center">
                        <ConnectButton />
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className="h-[calc(100vh-64px)] flex overflow-hidden">
            {/* Sidebar */}
            <div className="w-64 border-r border-zinc-900 overflow-y-auto">
                <div className="p-4">
                    <h1 className="text-xl font-light mb-6">Settings</h1>
                    <nav className="space-y-1">
                        {SECTIONS.map((section) => (
                            <button
                                key={section.id}
                                onClick={() => setActiveSection(section.id)}
                                className={`w-full text-left px-3 py-2 text-sm transition-colors ${activeSection === section.id
                                    ? 'bg-zinc-900 text-white'
                                    : 'text-zinc-500 hover:text-zinc-300'
                                    }`}
                            >
                                {section.title}
                            </button>
                        ))}
                    </nav>
                </div>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto">
                <div className="max-w-2xl p-8">
                    {activeSection === 'account' && (
                        <div className="space-y-8">
                            <div>
                                <h2 className="text-lg font-medium mb-2">Account</h2>
                                <p className="text-sm text-zinc-500">
                                    Manage your wallet connection and profile settings
                                </p>
                            </div>

                            <div className="space-y-4">
                                <div className="p-4 bg-zinc-900/50 border border-zinc-800">
                                    <div className="text-xs text-zinc-500 uppercase mb-2">Connected Wallet</div>
                                    <div className="font-mono text-sm text-zinc-300 mb-4">
                                        {address}
                                    </div>
                                    <button
                                        onClick={() => disconnect()}
                                        className="px-4 py-2 text-sm bg-red-900/30 text-red-400 hover:bg-red-900/50 transition-colors"
                                    >
                                        Disconnect Wallet
                                    </button>
                                </div>
                            </div>
                        </div>
                    )}

                    {activeSection === 'github' && (
                        <div className="space-y-8">
                            <div>
                                <h2 className="text-lg font-medium mb-2">GitHub Integration</h2>
                                <p className="text-sm text-zinc-500">
                                    Connect your GitHub account to import private repositories
                                </p>
                            </div>

                            {githubMessage && (
                                <div className={`p-4 border text-sm ${githubMessage.type === 'success'
                                    ? 'bg-green-900/20 border-green-900/50 text-green-400'
                                    : 'bg-red-900/20 border-red-900/50 text-red-400'
                                    }`}>
                                    {githubMessage.text}
                                </div>
                            )}

                            <div className="space-y-4">
                                {!githubUsername ? (
                                    <div className="p-6 bg-zinc-900/50 border border-zinc-800 text-center">
                                        <div className="w-12 h-12 mx-auto mb-4 rounded-full bg-zinc-800 flex items-center justify-center">
                                            <svg className="w-6 h-6 text-zinc-400" fill="currentColor" viewBox="0 0 24 24">
                                                <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                                            </svg>
                                        </div>
                                        <p className="text-zinc-400 mb-4">
                                            Connect your GitHub account to import private smart contract repositories
                                        </p>
                                        <button
                                            onClick={handleGitHubConnect}
                                            className="px-6 py-3 bg-zinc-800 text-white font-medium hover:bg-zinc-700 transition-colors inline-flex items-center gap-2"
                                            style={{ color: '#fff' }}
                                        >
                                            <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                                                <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                                            </svg>
                                            Connect GitHub
                                        </button>
                                    </div>
                                ) : (
                                    <div className="p-4 bg-zinc-900/50 border border-zinc-800">
                                        <div className="flex items-center justify-between">
                                            <div className="flex items-center gap-3">
                                                <div className="w-10 h-10 rounded-full bg-zinc-800 flex items-center justify-center">
                                                    <svg className="w-5 h-5 text-green-400" fill="currentColor" viewBox="0 0 24 24">
                                                        <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                                                    </svg>
                                                </div>
                                                <div>
                                                    <div className="text-sm font-medium text-green-400">GitHub Connected</div>
                                                    <div className="text-xs text-zinc-500">@{githubUsername}</div>
                                                </div>
                                            </div>
                                            <button
                                                onClick={handleGitHubDisconnect}
                                                className="px-3 py-1 text-sm text-red-400 hover:text-red-300 transition-colors"
                                            >
                                                Disconnect
                                            </button>
                                        </div>
                                    </div>
                                )}
                            </div>

                            <div className="space-y-4 pt-4 border-t border-zinc-900">
                                <h3 className="text-sm font-medium">Public Repositories</h3>
                                <p className="text-xs text-zinc-500">
                                    You can import from public repositories without connecting your GitHub account.
                                    Just paste the repository URL when creating a new project.
                                </p>
                            </div>

                            <div className="space-y-4">
                                <h3 className="text-sm font-medium">Repository Access</h3>
                                <p className="text-xs text-zinc-500">
                                    VeriPro only requests read access to repositories. We never modify your code.
                                </p>
                            </div>
                        </div>
                    )}

                    {activeSection === 'verification' && (
                        <div className="space-y-8">
                            <div>
                                <h2 className="text-lg font-medium mb-2">Verification Settings</h2>
                                <p className="text-sm text-zinc-500">
                                    Configure how verification and attestation works
                                </p>
                            </div>

                            <div className="space-y-4">
                                <div className="flex items-center justify-between p-4 bg-zinc-900/50 border border-zinc-800">
                                    <div>
                                        <div className="text-sm font-medium mb-1">Auto-generate Attestations</div>
                                        <div className="text-xs text-zinc-500">
                                            Automatically create on-chain attestations for successful verifications
                                        </div>
                                    </div>
                                    <button
                                        onClick={() => setSettings(prev => prev ? { ...prev, autoAttest: !prev.autoAttest } : prev)}
                                        className={`w-12 h-6 rounded-full transition-colors ${settings?.autoAttest ? 'bg-white' : 'bg-zinc-700'
                                            }`}
                                    >
                                        <div className={`w-5 h-5 rounded-full bg-black transition-transform ${settings?.autoAttest ? 'translate-x-6' : 'translate-x-0.5'
                                            }`} />
                                    </button>
                                </div>

                                <div className="p-4 bg-zinc-900/50 border border-zinc-800">
                                    <div className="text-sm font-medium mb-3">Default Prover Configuration</div>
                                    <div className="grid gap-3">
                                        <div>
                                            <label className="text-xs text-zinc-500 block mb-1">Timeout (seconds)</label>
                                            <input
                                                type="number"
                                                value={settings?.verificationTimeout || 300}
                                                onChange={(e) => setSettings(prev => prev ? { ...prev, verificationTimeout: parseInt(e.target.value) || 300 } : prev)}
                                                className="w-full px-3 py-2 bg-black border border-zinc-700 text-sm text-white focus:outline-none focus:border-zinc-500"
                                            />
                                        </div>
                                        <div>
                                            <label className="text-xs text-zinc-500 block mb-1">Max Workers</label>
                                            <input
                                                type="number"
                                                value={settings?.maxWorkers || 4}
                                                onChange={(e) => setSettings(prev => prev ? { ...prev, maxWorkers: parseInt(e.target.value) || 4 } : prev)}
                                                className="w-full px-3 py-2 bg-black border border-zinc-700 text-sm text-white focus:outline-none focus:border-zinc-500"
                                            />
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    )}

                    {activeSection === 'notifications' && (
                        <div className="space-y-8">
                            <div>
                                <h2 className="text-lg font-medium mb-2">Notifications</h2>
                                <p className="text-sm text-zinc-500">
                                    Configure how you receive updates about verifications
                                </p>
                            </div>

                            <div className="space-y-4">
                                <div className="flex items-center justify-between p-4 bg-zinc-900/50 border border-zinc-800">
                                    <div>
                                        <div className="text-sm font-medium mb-1">Email Notifications</div>
                                        <div className="text-xs text-zinc-500">
                                            Receive email updates when verifications complete
                                        </div>
                                    </div>
                                    <button
                                        onClick={() => setSettings(prev => prev ? { ...prev, emailNotifications: !prev.emailNotifications } : prev)}
                                        className={`w-12 h-6 rounded-full transition-colors ${settings?.emailNotifications ? 'bg-white' : 'bg-zinc-700'
                                            }`}
                                    >
                                        <div className={`w-5 h-5 rounded-full bg-black transition-transform ${settings?.emailNotifications ? 'translate-x-6' : 'translate-x-0.5'
                                            }`} />
                                    </button>
                                </div>

                                <div className="p-4 bg-zinc-900/50 border border-zinc-800">
                                    <div className="text-sm font-medium mb-3">Email Address</div>
                                    <input
                                        type="email"
                                        placeholder="your@email.com"
                                        value={settings?.email || ''}
                                        onChange={(e) => setSettings(prev => prev ? { ...prev, email: e.target.value } : prev)}
                                        className="w-full px-3 py-2 bg-black border border-zinc-700 text-sm text-white focus:outline-none focus:border-zinc-500"
                                    />
                                </div>
                            </div>
                        </div>
                    )}

                    {/* Save Button */}
                    <div className="mt-8 pt-6 border-t border-zinc-800">
                        <button
                            onClick={handleSave}
                            className="px-6 py-3 bg-white text-black font-medium hover:bg-zinc-200 transition-colors"
                        >
                            {saved ? '✓ Saved' : 'Save Changes'}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}
