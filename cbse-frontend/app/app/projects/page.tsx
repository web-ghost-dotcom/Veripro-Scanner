'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import { getProjects, type Project } from '../../lib/store';

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

export default function ProjectsPage() {
    const [projects, setProjects] = useState<Project[]>([]);

    useEffect(() => {
        // eslint-disable-next-line react-hooks/set-state-in-effect
        setProjects(getProjects());
    }, []);

    // Refresh on focus
    useEffect(() => {
        const handleFocus = () => setProjects(getProjects());
        window.addEventListener('focus', handleFocus);
        return () => window.removeEventListener('focus', handleFocus);
    }, []);

    return (
        <div className="p-8">
            <div className="max-w-6xl mx-auto">
                <div className="flex items-center justify-between mb-8">
                    <div>
                        <h1 className="text-2xl font-light mb-2">Projects</h1>
                        <p className="text-zinc-500 text-sm">All your verification projects</p>
                    </div>
                    <Link
                        href="/app/projects/new"
                        className="px-6 py-3 bg-white text-sm font-medium hover:bg-zinc-200 transition-colors inline-flex items-center"
                        style={{ color: '#000' }}
                    >
                        New Project
                    </Link>
                </div>

                {projects.length === 0 ? (
                    <div className="border border-zinc-900 bg-zinc-950 p-12 text-center">
                        <p className="text-zinc-500 mb-4">Your projects will appear here</p>
                        <Link
                            href="/app/projects/new"
                            className="text-sm text-white hover:underline"
                        >
                            Create your first project
                        </Link>
                    </div>
                ) : (
                    <div className="border border-zinc-900 bg-zinc-950 divide-y divide-zinc-900">
                        {projects.map((project) => (
                            <Link
                                key={project.id}
                                href={`/app/projects/${project.id}`}
                                className="block p-6 hover:bg-zinc-900/50 transition-colors"
                            >
                                <div className="flex items-center justify-between">
                                    <div>
                                        <h3 className="text-white font-medium mb-1">{project.name}</h3>
                                        <p className="text-sm text-zinc-500">
                                            {project.files.length} file{project.files.length !== 1 ? 's' : ''}
                                        </p>
                                    </div>
                                    <StatusBadge status={project.status} />
                                </div>
                            </Link>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}
