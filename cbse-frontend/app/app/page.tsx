'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import { useAuth } from './layout';
import { getProjects, deleteProject, type Project } from '../lib/store';
import { ConnectButton } from '@rainbow-me/rainbowkit';

function StatusBadge({ status }: { status: Project['status'] }) {
    const styles = {
        idle: 'bg-zinc-800 text-zinc-400',
        verifying: 'bg-yellow-900/50 text-yellow-400',
        passed: 'bg-green-900/50 text-green-400',
        failed: 'bg-red-900/50 text-red-400',
    };

    const labels = {
        idle: 'Not Verified',
        verifying: 'Verifying',
        passed: 'Passed',
        failed: 'Failed',
    };

    return (
        <span className={`px-2 py-1 text-xs rounded ${styles[status]}`}>
            {labels[status]}
        </span>
    );
}

function formatRelativeTime(isoString: string): string {
    const date = new Date(isoString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / (1000 * 60));
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    return `${diffDays}d ago`;
}

export default function DashboardPage() {
    const { isConnected } = useAuth();
    const [projects, setProjects] = useState<Project[]>([]);

    useEffect(() => {
        if (isConnected) {
            // Load projects from store
            // eslint-disable-next-line react-hooks/set-state-in-effect
            setProjects(getProjects());
        }
    }, [isConnected]);

    // Refresh projects when window gains focus
    useEffect(() => {
        const handleFocus = () => {
            if (isConnected) {
                setProjects(getProjects());
            }
        };
        window.addEventListener('focus', handleFocus);
        return () => window.removeEventListener('focus', handleFocus);
    }, [isConnected]);

    if (!isConnected) {
        return (
            <div className="flex items-center justify-center min-h-[calc(100vh-64px)]">
                <div className="text-center max-w-md">
                    <h1 className="text-3xl font-light mb-4">Welcome to VeriPro</h1>
                    <p className="text-zinc-500 mb-8">
                        Connect your wallet to access the verification workspace.
                        Manage projects, write specifications, and verify smart contracts.
                    </p>
                    <div className="flex justify-center">
                        <ConnectButton />
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className="p-8">
            <div className="max-w-6xl mx-auto">
                <div className="flex items-center justify-between mb-8">
                    <div>
                        <h1 className="text-2xl font-light mb-2">Dashboard</h1>
                        <p className="text-zinc-500 text-sm">Manage your smart contract verification projects</p>
                    </div>
                    <Link
                        href="/app/projects/new"
                        className="px-6 py-3 bg-white text-sm font-medium hover:bg-zinc-200 transition-colors inline-flex items-center"
                        style={{ color: '#000' }}
                    >
                        New Project
                    </Link>
                </div>

                {/* Stats */}
                <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-8">
                    {[
                        { label: 'Total Projects', value: projects.length },
                        { label: 'Verified', value: projects.filter(p => p.status === 'passed').length },
                        { label: 'Failed', value: projects.filter(p => p.status === 'failed').length },
                        { label: 'Pending', value: projects.filter(p => p.status === 'idle').length },
                    ].map((stat, i) => (
                        <div key={i} className="p-6 border border-zinc-900 bg-zinc-950">
                            <div className="text-3xl font-light mb-1">{stat.value}</div>
                            <div className="text-sm text-zinc-500">{stat.label}</div>
                        </div>
                    ))}
                </div>

                {/* Projects List */}
                <div className="border border-zinc-900 bg-zinc-950">
                    <div className="p-4 border-b border-zinc-900">
                        <h2 className="text-sm font-medium text-zinc-400 uppercase tracking-wide">Projects</h2>
                    </div>

                    {projects.length === 0 ? (
                        <div className="p-12 text-center">
                            <p className="text-zinc-500 mb-4">No projects yet</p>
                            <Link
                                href="/app/projects/new"
                                className="text-sm text-white hover:underline"
                            >
                                Create your first project
                            </Link>
                        </div>
                    ) : (
                        <div className="divide-y divide-zinc-900">
                            {projects.map((project) => {
                                const contractFiles = project.files.filter(f => f.type === 'contract').length;
                                const specFiles = project.files.filter(f => f.type === 'spec').length;
                                return (
                                    <div
                                        key={project.id}
                                        className="group flex items-center justify-between p-4 hover:bg-zinc-900/50 transition-colors"
                                    >
                                        <Link
                                            href={`/app/projects/${project.id}`}
                                            className="flex-1 flex items-center gap-4"
                                        >
                                            <div className="w-10 h-10 bg-zinc-900 rounded flex items-center justify-center text-zinc-500 text-sm font-mono">
                                                {project.name.charAt(0)}
                                            </div>
                                            <div>
                                                <div className="font-medium mb-1">{project.name}</div>
                                                <div className="text-xs text-zinc-600">
                                                    {contractFiles} contracts, {specFiles} specs
                                                </div>
                                            </div>
                                        </Link>
                                        <div className="flex items-center gap-4">
                                            <StatusBadge status={project.status} />
                                            <span className="text-xs text-zinc-600">
                                                {formatRelativeTime(project.updatedAt)}
                                            </span>
                                            <button
                                                onClick={(e) => {
                                                    e.preventDefault();
                                                    e.stopPropagation();
                                                    if (confirm(`Delete project "${project.name}"?`)) {
                                                        deleteProject(project.id);
                                                        setProjects(getProjects());
                                                    }
                                                }}
                                                className="opacity-0 group-hover:opacity-100 p-2 text-zinc-600 hover:text-red-400 transition-all"
                                                title="Delete project"
                                            >
                                                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                                                </svg>
                                            </button>
                                        </div>
                                    </div>
                                );
                            })}
                        </div>
                    )}
                </div>

                {/* Quick Actions */}
                <div className="mt-8 grid grid-cols-1 md:grid-cols-3 gap-4">
                    <Link
                        href="/app/verify"
                        className="p-6 border border-zinc-900 bg-zinc-950 hover:border-zinc-800 transition-colors"
                    >
                        <h3 className="font-medium mb-2">Quick Verify</h3>
                        <p className="text-sm text-zinc-500">
                            Paste code directly and run verification without creating a project.
                        </p>
                    </Link>
                    <Link
                        href="/app/projects/new?source=github"
                        className="p-6 border border-zinc-900 bg-zinc-950 hover:border-zinc-800 transition-colors"
                    >
                        <h3 className="font-medium mb-2">Import from GitHub</h3>
                        <p className="text-sm text-zinc-500">
                            Connect a repository and automatically sync your contracts.
                        </p>
                    </Link>
                    <Link
                        href="/docs"
                        className="p-6 border border-zinc-900 bg-zinc-950 hover:border-zinc-800 transition-colors"
                    >
                        <h3 className="font-medium mb-2">Learn Specifications</h3>
                        <p className="text-sm text-zinc-500">
                            Read the documentation on writing formal specifications.
                        </p>
                    </Link>
                </div>
            </div>
        </div>
    );
}
