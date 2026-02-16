'use client';

import { useState, useRef, useCallback, useEffect } from 'react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useAuth } from '../../layout';
import { createProject, getFileType, type ProjectFile } from '../../../lib/store';
import { ConnectButton } from '@rainbow-me/rainbowkit';

type SourceType = 'upload' | 'github' | 'paste';

interface GitHubRepo {
    id: number;
    name: string;
    full_name: string;
    html_url: string;
    description: string | null;
    private: boolean;
    default_branch: string;
    language: string | null;
    updated_at: string;
    owner: {
        login: string;
        avatar_url: string;
    };
}

export default function NewProjectPage() {
    const router = useRouter();
    const { isConnected } = useAuth();
    const [projectName, setProjectName] = useState('');
    const [sourceType, setSourceType] = useState<SourceType>('upload');
    const [githubUrl, setGithubUrl] = useState('');
    const [githubBranch, setGithubBranch] = useState('main');
    const [contractsPath, setContractsPath] = useState('src/');
    const [pastedCode, setPastedCode] = useState('');
    const [uploadedFiles, setUploadedFiles] = useState<{ name: string; content: string }[]>([]);
    const [isCreating, setIsCreating] = useState(false);
    const [isDragging, setIsDragging] = useState(false);
    const fileInputRef = useRef<HTMLInputElement>(null);

    // GitHub repos state
    const [userRepos, setUserRepos] = useState<GitHubRepo[]>([]);
    const [isLoadingRepos, setIsLoadingRepos] = useState(false);
    const [reposError, setReposError] = useState<string | null>(null);
    const [selectedRepo, setSelectedRepo] = useState<GitHubRepo | null>(null);
    const [repoSearchQuery, setRepoSearchQuery] = useState('');

    // Fetch user's repos when GitHub source is selected
    useEffect(() => {
        if (sourceType === 'github') {
            fetchUserRepos();
        }
    }, [sourceType]);

    const fetchUserRepos = async () => {
        setIsLoadingRepos(true);
        setReposError(null);

        try {
            const response = await fetch('/api/github/user-repos');
            const data = await response.json();

            if (response.ok && data.repos) {
                setUserRepos(data.repos);
            } else if (response.status === 401) {
                // Not authenticated - that's fine, they can still enter URL manually
                setUserRepos([]);
            } else {
                setReposError(data.error || 'Failed to load repositories');
            }
        } catch (err) {
            // Silently fail - user can still enter URL manually
            setUserRepos([]);
        } finally {
            setIsLoadingRepos(false);
        }
    };

    const handleSelectRepo = (repo: GitHubRepo) => {
        setSelectedRepo(repo);
        setGithubUrl(repo.html_url);
        setGithubBranch(repo.default_branch);
        if (!projectName) {
            setProjectName(repo.name);
        }
    };

    const filteredRepos = userRepos.filter(repo =>
        repo.name.toLowerCase().includes(repoSearchQuery.toLowerCase()) ||
        repo.full_name.toLowerCase().includes(repoSearchQuery.toLowerCase())
    );

    const handleFileRead = useCallback((file: File): Promise<{ name: string; content: string }> => {
        return new Promise((resolve, reject) => {
            const reader = new FileReader();
            reader.onload = (e) => {
                resolve({
                    name: file.name,
                    content: e.target?.result as string || '',
                });
            };
            reader.onerror = reject;
            reader.readAsText(file);
        });
    }, []);

    const handleFilesSelected = useCallback(async (files: FileList | File[]) => {
        const fileArray = Array.from(files).filter(f =>
            f.name.endsWith('.sol') || f.name.endsWith('.json')
        );

        const readFiles = await Promise.all(fileArray.map(handleFileRead));
        setUploadedFiles(prev => [...prev, ...readFiles]);
    }, [handleFileRead]);

    const handleDrop = useCallback((e: React.DragEvent) => {
        e.preventDefault();
        setIsDragging(false);

        if (e.dataTransfer.files.length > 0) {
            handleFilesSelected(e.dataTransfer.files);
        }
    }, [handleFilesSelected]);

    const handleDragOver = useCallback((e: React.DragEvent) => {
        e.preventDefault();
        setIsDragging(true);
    }, []);

    const handleDragLeave = useCallback((e: React.DragEvent) => {
        e.preventDefault();
        setIsDragging(false);
    }, []);

    const removeFile = (index: number) => {
        setUploadedFiles(prev => prev.filter((_, i) => i !== index));
    };

    const [githubError, setGithubError] = useState<string | null>(null);
    const [isFetchingGithub, setIsFetchingGithub] = useState(false);

    const fetchGitHubFiles = async (): Promise<{ name: string; path: string; content: string }[]> => {
        setIsFetchingGithub(true);
        setGithubError(null);

        try {
            const response = await fetch('/api/github/repos', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    githubUrl,
                    branch: githubBranch,
                    contractsPath,
                }),
            });

            const data = await response.json();

            if (!response.ok) {
                throw new Error(data.error || 'Failed to fetch repository');
            }

            return data.files || [];
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            setGithubError(message);
            return [];
        } finally {
            setIsFetchingGithub(false);
        }
    };

    const handleCreate = async () => {
        if (!projectName.trim()) return;

        setIsCreating(true);

        try {
            // Build files array based on source type
            let files: Omit<ProjectFile, 'id'>[] = [];

            if (sourceType === 'upload') {
                files = uploadedFiles.map(f => ({
                    name: f.name,
                    path: `src/${f.name}`,
                    content: f.content,
                    type: getFileType(f.name),
                }));
            } else if (sourceType === 'github' && githubUrl.trim()) {
                // Fetch files from GitHub
                const githubFiles = await fetchGitHubFiles();
                if (githubFiles.length === 0 && githubError) {
                    setIsCreating(false);
                    return;
                }
                files = githubFiles.map(f => ({
                    name: f.name,
                    path: f.path,
                    content: f.content,
                    type: getFileType(f.name),
                }));
            } else if (sourceType === 'paste' && pastedCode.trim()) {
                // Extract contract name from code or use default
                const contractMatch = pastedCode.match(/contract\s+(\w+)/);
                const fileName = contractMatch ? `${contractMatch[1]}.sol` : 'Contract.sol';
                files = [{
                    name: fileName,
                    path: `src/${fileName}`,
                    content: pastedCode,
                    type: 'contract',
                }];
            }

            if (files.length === 0) {
                setGithubError('No Solidity files found. Please check the repository URL and path.');
                setIsCreating(false);
                return;
            }

            // Create project in store
            const project = createProject({
                name: projectName,
                source: sourceType,
                githubUrl: sourceType === 'github' ? githubUrl : undefined,
                githubBranch: sourceType === 'github' ? githubBranch : undefined,
                files: files.map((f, i) => ({ ...f, id: `file_${i}` })),
            });

            // Redirect to project page
            router.push(`/app/projects/${project.id}`);
        } catch (err) {
            console.error('Failed to create project:', err);
            setIsCreating(false);
        }
    };

    if (!isConnected) {
        return (
            <div className="flex items-center justify-center min-h-[calc(100vh-64px)]">
                <div className="text-center max-w-md">
                    <h1 className="text-3xl font-light mb-4">Create New Project</h1>
                    <p className="text-zinc-500 mb-8">
                        Connect your wallet to create a new verification project.
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
            <div className="max-w-2xl mx-auto">
                <div className="mb-8">
                    <Link href="/app" className="text-sm text-zinc-500 hover:text-zinc-300 transition-colors">
                        Back to Dashboard
                    </Link>
                </div>

                <h1 className="text-2xl font-light mb-8">Create New Project</h1>

                <div className="space-y-8">
                    {/* Project Name */}
                    <div>
                        <label className="block text-sm text-zinc-400 mb-2">Project Name</label>
                        <input
                            type="text"
                            value={projectName}
                            onChange={(e) => setProjectName(e.target.value)}
                            placeholder="My Token Contract"
                            className="w-full px-4 py-3 bg-zinc-950 border border-zinc-900 text-white placeholder-zinc-600 focus:outline-none focus:border-zinc-700 transition-colors"
                        />
                    </div>

                    {/* Source Selection */}
                    <div>
                        <label className="block text-sm text-zinc-400 mb-4">Contract Source</label>
                        <div className="grid grid-cols-3 gap-4">
                            {[
                                { id: 'upload', label: 'Upload Files' },
                                { id: 'github', label: 'GitHub Repository' },
                                { id: 'paste', label: 'Paste Code' },
                            ].map((option) => (
                                <button
                                    key={option.id}
                                    onClick={() => setSourceType(option.id as SourceType)}
                                    className={`p-4 border text-sm transition-colors ${sourceType === option.id
                                        ? 'border-white bg-zinc-900 text-white'
                                        : 'border-zinc-900 text-zinc-500 hover:border-zinc-800'
                                        }`}
                                >
                                    {option.label}
                                </button>
                            ))}
                        </div>
                    </div>

                    {/* Source-specific inputs */}
                    {sourceType === 'upload' && (
                        <div>
                            <div
                                onDrop={handleDrop}
                                onDragOver={handleDragOver}
                                onDragLeave={handleDragLeave}
                                className={`border-2 border-dashed p-12 text-center transition-colors ${isDragging
                                    ? 'border-white bg-zinc-900/50'
                                    : 'border-zinc-900'
                                    }`}
                            >
                                <input
                                    ref={fileInputRef}
                                    type="file"
                                    multiple
                                    accept=".sol,.json"
                                    onChange={(e) => e.target.files && handleFilesSelected(e.target.files)}
                                    className="hidden"
                                />
                                <p className="text-zinc-500 mb-4">
                                    {isDragging
                                        ? 'Drop files here...'
                                        : 'Drag and drop Solidity files here'
                                    }
                                </p>
                                <button
                                    onClick={() => fileInputRef.current?.click()}
                                    className="px-6 py-2 bg-zinc-900 hover:bg-zinc-800 text-sm transition-colors"
                                >
                                    Browse Files
                                </button>
                                <p className="text-xs text-zinc-600 mt-4">
                                    Supports .sol and .json files
                                </p>
                            </div>

                            {/* Uploaded files list */}
                            {uploadedFiles.length > 0 && (
                                <div className="mt-4 space-y-2">
                                    <div className="text-sm text-zinc-400">Uploaded Files:</div>
                                    {uploadedFiles.map((file, i) => (
                                        <div
                                            key={i}
                                            className="flex items-center justify-between p-3 bg-zinc-900/50 border border-zinc-800"
                                        >
                                            <span className="text-sm font-mono text-zinc-300">{file.name}</span>
                                            <button
                                                onClick={() => removeFile(i)}
                                                className="text-xs text-zinc-500 hover:text-red-400 transition-colors"
                                            >
                                                Remove
                                            </button>
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                    )}

                    {sourceType === 'github' && (
                        <div className="space-y-4">
                            {/* Repository Browser */}
                            {userRepos.length > 0 && (
                                <div className="p-4 border border-zinc-900 bg-zinc-950">
                                    <div className="flex items-center justify-between mb-3">
                                        <label className="text-sm text-zinc-400">Your Repositories</label>
                                        <input
                                            type="text"
                                            placeholder="Search repos..."
                                            value={repoSearchQuery}
                                            onChange={(e) => setRepoSearchQuery(e.target.value)}
                                            className="px-3 py-1 text-sm bg-black border border-zinc-800 text-white placeholder-zinc-600 focus:outline-none focus:border-zinc-700"
                                        />
                                    </div>
                                    <div className="max-h-64 overflow-y-auto space-y-1">
                                        {filteredRepos.map((repo) => (
                                            <button
                                                key={repo.id}
                                                onClick={() => handleSelectRepo(repo)}
                                                className={`w-full text-left p-3 transition-colors flex items-center justify-between ${selectedRepo?.id === repo.id
                                                    ? 'bg-zinc-800 border border-zinc-700'
                                                    : 'bg-zinc-900/50 border border-transparent hover:bg-zinc-900'
                                                    }`}
                                            >
                                                <div className="flex items-center gap-3">
                                                    <div className="text-sm">
                                                        <span className="text-white">{repo.name}</span>
                                                        {repo.private && (
                                                            <span className="ml-2 px-1.5 py-0.5 text-xs bg-yellow-900/30 text-yellow-500 rounded">Private</span>
                                                        )}
                                                    </div>
                                                </div>
                                                <span className="text-xs text-zinc-600">{repo.default_branch}</span>
                                            </button>
                                        ))}
                                    </div>
                                </div>
                            )}

                            {isLoadingRepos && (
                                <div className="p-4 border border-zinc-900 bg-zinc-950 text-center">
                                    <div className="w-5 h-5 border-2 border-zinc-600 border-t-white rounded-full animate-spin mx-auto mb-2"></div>
                                    <p className="text-sm text-zinc-500">Loading your repositories...</p>
                                </div>
                            )}

                            {userRepos.length === 0 && !isLoadingRepos && (
                                <div className="p-4 border border-zinc-900 bg-zinc-950 text-center">
                                    <p className="text-sm text-zinc-500 mb-2">
                                        {reposError || 'Connect GitHub in Settings to browse your repos, or enter a URL below'}
                                    </p>
                                    <Link href="/app/settings" className="text-sm text-white hover:underline">
                                        Go to Settings →
                                    </Link>
                                </div>
                            )}

                            {/* Manual URL Input */}
                            <div className="p-4 border border-zinc-900 bg-zinc-950 space-y-4">
                                <div>
                                    <label className="block text-sm text-zinc-400 mb-2">
                                        {userRepos.length > 0 ? 'Or enter repository URL manually' : 'Repository URL'}
                                    </label>
                                    <input
                                        type="text"
                                        value={githubUrl}
                                        onChange={(e) => {
                                            setGithubUrl(e.target.value);
                                            setSelectedRepo(null);
                                        }}
                                        placeholder="https://github.com/username/repo"
                                        className="w-full px-4 py-3 bg-black border border-zinc-900 text-white placeholder-zinc-600 focus:outline-none focus:border-zinc-700 transition-colors"
                                    />
                                </div>
                                <div className="grid grid-cols-2 gap-4">
                                    <div>
                                        <label className="block text-sm text-zinc-400 mb-2">Branch</label>
                                        <input
                                            type="text"
                                            value={githubBranch}
                                            onChange={(e) => setGithubBranch(e.target.value)}
                                            placeholder="main"
                                            className="w-full px-4 py-3 bg-black border border-zinc-900 text-white placeholder-zinc-600 focus:outline-none focus:border-zinc-700 transition-colors"
                                        />
                                    </div>
                                    <div>
                                        <label className="block text-sm text-zinc-400 mb-2">Contracts Path</label>
                                        <input
                                            type="text"
                                            value={contractsPath}
                                            onChange={(e) => setContractsPath(e.target.value)}
                                            placeholder="src/"
                                            className="w-full px-4 py-3 bg-black border border-zinc-900 text-white placeholder-zinc-600 focus:outline-none focus:border-zinc-700 transition-colors"
                                        />
                                    </div>
                                </div>
                            </div>

                            {githubError && (
                                <div className="p-4 bg-red-900/20 border border-red-900/50 text-red-400 text-sm">
                                    {githubError}
                                </div>
                            )}

                            {isFetchingGithub && (
                                <div className="p-4 bg-zinc-900/50 border border-zinc-800 text-zinc-400 text-sm flex items-center gap-3">
                                    <div className="w-4 h-4 border-2 border-zinc-600 border-t-white rounded-full animate-spin"></div>
                                    Fetching Solidity files from repository...
                                </div>
                            )}

                            <p className="text-xs text-zinc-600">
                                Enter a public repository URL. We&apos;ll automatically find and import all .sol files.
                            </p>
                        </div>
                    )}

                    {sourceType === 'paste' && (
                        <div>
                            <label className="block text-sm text-zinc-400 mb-2">Paste Solidity Code</label>
                            <textarea
                                value={pastedCode}
                                onChange={(e) => setPastedCode(e.target.value)}
                                rows={12}
                                placeholder={`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MyContract {
    // ...
}`}
                                className="w-full px-4 py-3 bg-zinc-950 border border-zinc-900 text-white placeholder-zinc-600 focus:outline-none focus:border-zinc-700 transition-colors font-mono text-sm resize-none"
                            />
                        </div>
                    )}

                    {/* Create Button */}
                    <div className="flex gap-4">
                        <button
                            onClick={handleCreate}
                            disabled={!projectName.trim() || isCreating}
                            className={`px-8 py-4 font-medium transition-colors ${projectName.trim() && !isCreating
                                ? 'bg-white text-black hover:bg-zinc-200'
                                : 'bg-zinc-800 text-zinc-600 cursor-not-allowed'
                                }`}
                        >
                            {isCreating ? 'Creating...' : 'Create Project'}
                        </button>
                        <Link
                            href="/app"
                            className="px-8 py-4 border border-zinc-900 text-zinc-400 hover:border-zinc-800 hover:text-zinc-300 transition-colors"
                        >
                            Cancel
                        </Link>
                    </div>
                </div>
            </div>
        </div>
    );
}
