'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import { useParams, useRouter } from 'next/navigation';
import { useAuth } from '../../layout';
import { useAccount } from 'wagmi';
import { ConnectButton } from '@rainbow-me/rainbowkit';
import {
    getProject,
    updateProject,
    addFileToProject,
    updateFileInProject,
    deleteFileFromProject,
    addResult,
    updateResult,
    getFileType,
    type Project,
    type ProjectFile
} from '../../../lib/store';
import { useCommitAttestation } from '../../../lib/useAttestation';
import VeriProAI from '../../../components/VeriProAI';
import SyntaxEditor from '../../../components/SyntaxEditor';

function FileList({
    files,
    selectedFileId,
    onSelect,
    onDelete
}: {
    files: ProjectFile[];
    selectedFileId: string | null;
    onSelect: (file: ProjectFile) => void;
    onDelete?: (file: ProjectFile) => void;
}) {
    const contracts = files.filter(f => f.type === 'contract');
    const specs = files.filter(f => f.type === 'spec');
    const others = files.filter(f => f.type === 'other');

    const FileItem = ({ file }: { file: ProjectFile }) => (
        <div
            className={`group flex items-center justify-between px-2 py-1.5 text-sm transition-colors ${selectedFileId === file.id
                ? 'bg-zinc-900 text-white'
                : 'text-zinc-400 hover:bg-zinc-900/50'
                }`}
        >
            <button
                onClick={() => onSelect(file)}
                className="flex-1 text-left truncate"
            >
                {file.name}
            </button>
            {onDelete && file.type !== 'contract' && (
                <button
                    onClick={(e) => {
                        e.stopPropagation();
                        onDelete(file);
                    }}
                    className="opacity-0 group-hover:opacity-100 p-1 text-zinc-600 hover:text-red-400 transition-all"
                    title="Delete file"
                >
                    <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            )}
        </div>
    );

    return (
        <div className="space-y-4">
            {contracts.length > 0 && (
                <div>
                    <div className="px-2 py-1.5 text-xs text-zinc-600 uppercase">Contracts</div>
                    {contracts.map((file) => (
                        <FileItem key={file.id} file={file} />
                    ))}
                </div>
            )}
            {specs.length > 0 && (
                <div>
                    <div className="px-2 py-1.5 text-xs text-zinc-600 uppercase">Specifications</div>
                    {specs.map((file) => (
                        <FileItem key={file.id} file={file} />
                    ))}
                </div>
            )}
            {others.length > 0 && (
                <div>
                    <div className="px-2 py-1.5 text-xs text-zinc-600 uppercase">Other</div>
                    {others.map((file) => (
                        <FileItem key={file.id} file={file} />
                    ))}
                </div>
            )}
            {files.length === 0 && (
                <div className="p-4 text-center text-zinc-600 text-sm">
                    No files yet. Add a specification to get started.
                </div>
            )}
        </div>
    );
}

export default function ProjectDetailPage() {
    const params = useParams();
    const router = useRouter();
    const { isConnected } = useAuth();
    const [project, setProject] = useState<Project | null>(null);
    const [loading, setLoading] = useState(true);
    const [selectedFile, setSelectedFile] = useState<ProjectFile | null>(null);
    const [editedContent, setEditedContent] = useState('');
    const [specContent, setSpecContent] = useState('');
    const [activeTab, setActiveTab] = useState<'contract' | 'spec'>('contract');
    const [isVerifying, setIsVerifying] = useState(false);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const [verificationResult, setVerificationResult] = useState<any>(null);
    const [showAddSpec, setShowAddSpec] = useState(false);
    const [newSpecContent, setNewSpecContent] = useState('');
    const [lastResultId, setLastResultId] = useState<string | null>(null);
    const [rightPanelTab, setRightPanelTab] = useState<'results' | 'ai'>('results');
    const [autoPrompt, setAutoPrompt] = useState<string | null>(null);
    const { chain } = useAccount();

    // On-chain attestation hook
    const {
        commitAttestation,
        hash: attestationTxHash,
        isPending: isAttesting,
        isConfirming: isConfirmingAttestation,
        isSuccess: attestationSuccess,
        error: attestationError
    } = useCommitAttestation();

    // Load project from store
    useEffect(() => {
        if (params.id) {
            const proj = getProject(params.id as string);
            if (proj) {
                setProject(proj);
                // Select first contract file if exists
                const firstContract = proj.files.find(f => f.type === 'contract');
                const firstSpec = proj.files.find(f => f.type === 'spec');
                if (firstContract) {
                    setSelectedFile(firstContract);
                    setEditedContent(firstContract.content);
                }
                if (firstSpec) {
                    setSpecContent(firstSpec.content);
                }
            }
            setLoading(false);
        }
    }, [params.id]);

    const handleFileSelect = (file: ProjectFile) => {
        setSelectedFile(file);
        if (file.type === 'spec') {
            setSpecContent(file.content);
            setActiveTab('spec');
        } else {
            setEditedContent(file.content);
            setActiveTab('contract');
        }
    };

    const handleDeleteFile = (file: ProjectFile) => {
        if (!project) return;
        if (confirm(`Delete ${file.name}?`)) {
            deleteFileFromProject(project.id, file.id);
            const updated = getProject(project.id);
            if (updated) {
                setProject(updated);
                // If deleted file was selected, select another
                if (selectedFile?.id === file.id) {
                    const firstFile = updated.files[0];
                    if (firstFile) {
                        handleFileSelect(firstFile);
                    } else {
                        setSelectedFile(null);
                    }
                }
            }
        }
    };

    const handleSaveFile = () => {
        if (!project || !selectedFile) return;
        const content = activeTab === 'contract' ? editedContent : specContent;
        updateFileInProject(project.id, selectedFile.id, content);
        // Refresh project
        const updated = getProject(project.id);
        if (updated) setProject(updated);
    };

    const handleAddSpec = () => {
        if (!project || !newSpecContent.trim()) return;

        // Extract test name from content or use default
        const testMatch = newSpecContent.match(/contract\s+(\w+)/);
        const fileName = testMatch ? `${testMatch[1]}.t.sol` : 'Spec.t.sol';

        addFileToProject(project.id, {
            name: fileName,
            path: `test/${fileName}`,
            content: newSpecContent,
            type: 'spec',
        });

        // Refresh project
        const updated = getProject(project.id);
        if (updated) {
            setProject(updated);
            const newSpec = updated.files.find(f => f.name === fileName);
            if (newSpec) {
                setSelectedFile(newSpec);
                setSpecContent(newSpec.content);
                setActiveTab('spec');
            }
        }
        setShowAddSpec(false);
        setNewSpecContent('');
    };

    const handleGenerateSpecs = () => {
        setRightPanelTab('ai');
        setAutoPrompt("Analyze this contract and generate a comprehensive formal specification using Foundry test invariants. Include checks for potential security vulnerabilities and edge cases.");
    };

    const handleVerify = async () => {
        if (!project) return;

        // Get contract file from project
        const contractFile = project.files.find(f => f.type === 'contract');

        if (!contractFile) {
            setVerificationResult({ status: 'Error', message: 'No contract file found' });
            return;
        }

        // Use current editor content (which may not be saved yet)
        // editedContent tracks the contract, specContent tracks the spec
        // Both should always use the state values, not saved file content
        const currentContractContent = editedContent || contractFile.content;
        const currentSpecContent = specContent;

        // Check if spec content exists
        if (!currentSpecContent.trim()) {
            setVerificationResult({
                status: 'Error',
                message: 'No specification provided. Please add a specification using the "Specification" tab or click "Generate Specs" to create one with AI.'
            });
            return;
        }

        setIsVerifying(true);
        setVerificationResult(null);
        updateProject(project.id, { status: 'verifying' });

        try {
            const res = await fetch('/api/verify', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    contract_source: currentContractContent,
                    spec_source: currentSpecContent,
                    contract_name: contractFile.name.replace('.sol', '')
                })
            });

            if (res.ok) {
                const data = await res.json();
                setVerificationResult(data);

                // Save result to store
                const newResult = addResult({
                    projectId: project.id,
                    contractName: contractFile.name.replace('.sol', ''),
                    status: data.status === 'Success' ? 'verified' : 'failed',
                    message: data.message,
                    bytecodeHash: data.attestation?.bytecode_hash,
                    specHash: data.attestation?.spec_hash,
                    signature: data.attestation?.signature,
                    properties: [],
                    attestation: data.attestation,
                });
                setLastResultId(newResult.id);

                // Update project status
                updateProject(project.id, {
                    status: data.status === 'Success' ? 'passed' : 'failed'
                });
            } else {
                setVerificationResult({ status: 'Error', message: 'Verification failed' });
                updateProject(project.id, { status: 'failed' });
            }
        } catch (err) {
            setVerificationResult({ status: 'Error', message: 'Connection failed' });
            updateProject(project.id, { status: 'failed' });
        } finally {
            setIsVerifying(false);
            // Refresh project state
            const updated = getProject(project.id);
            if (updated) setProject(updated);
        }
    };

    const handleCommitOnChain = async () => {
        if (!verificationResult?.attestation) return;

        const { attestation } = verificationResult;
        try {
            await commitAttestation({
                resultHash: attestation.result_hash,
                passed: verificationResult.status === 'Success',
                contractHash: attestation.bytecode_hash || attestation.result_hash,
                signature: attestation.signature,
            });
        } catch (err) {
            console.error('Failed to commit attestation:', err);
        }
    };

    // Update result when attestation is confirmed on-chain
    useEffect(() => {
        if (attestationSuccess && attestationTxHash && lastResultId) {
            updateResult(lastResultId, {
                onchainTxHash: attestationTxHash,
                onchainAttested: true,
            });
        }
    }, [attestationSuccess, attestationTxHash, lastResultId]);

    if (loading) {
        return (
            <div className="flex items-center justify-center min-h-[calc(100vh-64px)]">
                <div className="w-8 h-8 border-2 border-white border-t-transparent rounded-full animate-spin" />
            </div>
        );
    }

    if (!project) {
        return (
            <div className="flex items-center justify-center min-h-[calc(100vh-64px)]">
                <div className="text-center max-w-md">
                    <h1 className="text-3xl font-light mb-4">Project Not Found</h1>
                    <p className="text-zinc-500 mb-8">
                        This project does not exist or has been deleted.
                    </p>
                    <Link
                        href="/app"
                        className="px-8 py-4 bg-white text-black font-medium hover:bg-zinc-200 transition-colors inline-block"
                    >
                        Back to Dashboard
                    </Link>
                </div>
            </div>
        );
    }

    if (!isConnected) {
        return (
            <div className="flex items-center justify-center min-h-[calc(100vh-64px)]">
                <div className="text-center max-w-md">
                    <h1 className="text-3xl font-light mb-4">Project Details</h1>
                    <p className="text-zinc-500 mb-8">
                        Connect your wallet to view this project.
                    </p>
                    <div className="flex justify-center">
                        <ConnectButton />
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className="h-[calc(100vh-64px)] flex flex-col">
            {/* Header */}
            <div className="flex items-center justify-between px-6 py-4 border-b border-zinc-900">
                <div className="flex items-center gap-4">
                    <Link href="/app" className="text-sm text-zinc-500 hover:text-zinc-300 transition-colors">
                        Dashboard
                    </Link>
                    <span className="text-zinc-700">/</span>
                    <span className="text-white">{project.name}</span>
                </div>
                <div className="flex items-center gap-4">
                    <button
                        onClick={handleSaveFile}
                        className="px-4 py-2 text-sm text-zinc-400 border border-zinc-800 hover:border-zinc-700 transition-colors"
                    >
                        Save
                    </button>
                    <button
                        onClick={handleGenerateSpecs}
                        className="px-4 py-2 text-sm text-purple-400 border border-purple-900/50 hover:bg-purple-900/20 transition-colors flex items-center gap-2"
                    >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
                        </svg>
                        Generate Specs
                    </button>
                    <button
                        onClick={handleVerify}
                        disabled={isVerifying}
                        className={`px-6 py-2 text-sm font-medium transition-colors ${isVerifying
                            ? 'bg-zinc-800 text-zinc-500 cursor-not-allowed'
                            : 'bg-white text-black hover:bg-zinc-200'
                            }`}
                    >
                        {isVerifying ? 'Verifying...' : 'Run Verification'}
                    </button>
                </div>
            </div>

            {/* Main Content */}
            <div className="flex-1 flex overflow-hidden">
                {/* File Explorer */}
                <div className="w-64 border-r border-zinc-900 overflow-y-auto flex flex-col">
                    <div className="p-3 border-b border-zinc-900 flex items-center justify-between">
                        <span className="text-xs text-zinc-500 uppercase tracking-wide">Files</span>
                        <button
                            onClick={() => setShowAddSpec(true)}
                            className="text-xs text-zinc-500 hover:text-white transition-colors"
                        >
                            + Add Spec
                        </button>
                    </div>
                    <div className="flex-1 overflow-y-auto">
                        <FileList
                            files={project.files}
                            selectedFileId={selectedFile?.id || null}
                            onSelect={handleFileSelect}
                            onDelete={handleDeleteFile}
                        />
                    </div>
                </div>

                {/* Editor */}
                <div className="flex-1 flex flex-col overflow-hidden">
                    {/* Tabs */}
                    <div className="flex border-b border-zinc-900">
                        <button
                            onClick={() => setActiveTab('contract')}
                            className={`px-4 py-3 text-sm transition-colors ${activeTab === 'contract'
                                ? 'bg-zinc-900 text-white border-b border-white'
                                : 'text-zinc-500 hover:text-zinc-300'
                                }`}
                        >
                            Contract
                        </button>
                        <button
                            onClick={() => setActiveTab('spec')}
                            className={`px-4 py-3 text-sm transition-colors ${activeTab === 'spec'
                                ? 'bg-zinc-900 text-white border-b border-white'
                                : 'text-zinc-500 hover:text-zinc-300'
                                }`}
                        >
                            Specification
                        </button>
                    </div>

                    {/* Code Editor */}
                    <div className="flex-1 overflow-hidden relative">
                        {project.files.length === 0 ? (
                            <div className="flex items-center justify-center h-full text-zinc-600">
                                <div className="text-center">
                                    <p className="mb-4">No files in this project</p>
                                    <button
                                        onClick={() => setShowAddSpec(true)}
                                        className="text-sm text-white hover:underline"
                                    >
                                        Add a specification
                                    </button>
                                </div>
                            </div>
                        ) : (
                            <SyntaxEditor
                                value={activeTab === 'contract' ? editedContent : specContent}
                                onChange={(value) => {
                                    if (activeTab === 'contract') {
                                        setEditedContent(value);
                                    } else {
                                        setSpecContent(value);
                                    }
                                }}
                                placeholder={activeTab === 'contract'
                                    ? '// Select a contract file or paste code here...'
                                    : '// Write your specification here...'
                                }
                            />
                        )}
                    </div>
                </div>

                {/* Results Panel */}
                <div className="w-80 border-l border-zinc-900 flex flex-col">
                    <div className="flex border-b border-zinc-900">
                        <button
                            onClick={() => setRightPanelTab('results')}
                            className={`flex-1 p-3 text-xs uppercase tracking-wide transition-colors ${rightPanelTab === 'results'
                                ? 'text-white border-b-2 border-white bg-zinc-900/50'
                                : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/30'
                                }`}
                        >
                            Results
                        </button>
                        <button
                            onClick={() => setRightPanelTab('ai')}
                            className={`flex-1 p-3 text-xs uppercase tracking-wide transition-colors ${rightPanelTab === 'ai'
                                ? 'text-white border-b-2 border-white bg-zinc-900/50'
                                : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/30'
                                }`}
                        >
                            VeriPro AI
                        </button>
                    </div>

                    <div className="flex-1 overflow-hidden relative">
                        {rightPanelTab === 'results' ? (
                            <div className="flex-1 overflow-auto p-4 h-full">
                                {!verificationResult && !isVerifying && (
                                    <div className="text-center text-zinc-600 text-sm py-8">
                                        Run verification to see results
                                    </div>
                                )}

                                {isVerifying && (
                                    <div className="text-center py-8">
                                        <div className="w-8 h-8 border-2 border-white border-t-transparent rounded-full animate-spin mx-auto mb-4" />
                                        <p className="text-sm text-zinc-500">Running verification...</p>
                                    </div>
                                )}

                                {verificationResult && (
                                    <div className="space-y-4">
                                        <div className={`p-4 rounded ${verificationResult.status === 'Success'
                                            ? 'bg-green-900/20 border border-green-900/50'
                                            : 'bg-red-900/20 border border-red-900/50'
                                            }`}>
                                            <div className={`text-sm font-medium mb-2 ${verificationResult.status === 'Success'
                                                ? 'text-green-400'
                                                : 'text-red-400'
                                                }`}>
                                                {verificationResult.status}
                                            </div>
                                            <p className="text-xs text-zinc-400">
                                                {verificationResult.message}
                                            </p>
                                        </div>

                                        {verificationResult.attestation && (
                                            <div className="p-4 bg-zinc-900/50 rounded">
                                                <div className="text-xs text-zinc-500 uppercase mb-2">Attestation</div>
                                                <div className="text-xs font-mono text-zinc-400 break-all">
                                                    {verificationResult.attestation.signature?.slice(0, 40)}...
                                                </div>
                                            </div>
                                        )}

                                        {/* On-chain attestation button */}
                                        {verificationResult.status === 'Success' && verificationResult.attestation && (
                                            <div className="space-y-2">
                                                {attestationSuccess ? (
                                                    <div className="p-3 bg-green-900/20 border border-green-900/50 rounded">
                                                        <div className="flex items-center gap-2 text-green-400 text-xs">
                                                            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                                                            </svg>
                                                            Attested On-Chain
                                                        </div>
                                                        {attestationTxHash && (
                                                            <a
                                                                href={`${chain?.blockExplorers?.default.url || 'https://etherscan.io'}/tx/${attestationTxHash}`}
                                                                target="_blank"
                                                                rel="noopener noreferrer"
                                                                className="text-xs text-zinc-500 hover:text-zinc-300 mt-1 block truncate"
                                                            >
                                                                Tx: {attestationTxHash.slice(0, 20)}...
                                                            </a>
                                                        )}
                                                    </div>
                                                ) : (
                                                    <button
                                                        onClick={handleCommitOnChain}
                                                        disabled={isAttesting || isConfirmingAttestation}
                                                        className={`w-full px-4 py-3 text-sm font-medium transition-colors rounded ${isAttesting || isConfirmingAttestation
                                                            ? 'bg-zinc-800 text-zinc-500 cursor-not-allowed'
                                                            : 'bg-blue-600 text-white hover:bg-blue-700'
                                                            }`}
                                                    >
                                                        {isAttesting ? 'Confirm in Wallet...' :
                                                            isConfirmingAttestation ? 'Confirming...' :
                                                                '⛓️ Commit Attestation On-Chain'}
                                                    </button>
                                                )}
                                                {attestationError && (
                                                    <p className="text-xs text-red-400">{attestationError.message}</p>
                                                )}
                                            </div>
                                        )}

                                        {/* Copy button */}
                                        <button
                                            onClick={() => {
                                                if (verificationResult.attestation) {
                                                    navigator.clipboard.writeText(JSON.stringify(verificationResult.attestation, null, 2));
                                                }
                                            }}
                                            className="w-full px-4 py-2 text-xs bg-zinc-900 text-zinc-400 hover:text-white transition-colors"
                                        >
                                            Copy Attestation
                                        </button>
                                    </div>
                                )}
                            </div>
                        ) : (
                            <VeriProAI
                                contractCode={activeTab === 'contract' ? editedContent : (project.files.find(f => f.type === 'contract')?.content || '')}
                                contractName={project.files.find(f => f.type === 'contract')?.name.replace('.sol', '')}
                                onApplyCode={(code) => {
                                    if (activeTab === 'spec') {
                                        setSpecContent(prev => prev + '\n\n' + code);
                                    } else {
                                        setShowAddSpec(true);
                                        setNewSpecContent(code);
                                    }
                                }}
                                autoPrompt={autoPrompt}
                                onPromptHandled={() => setAutoPrompt(null)}
                            />
                        )}
                    </div>
                </div>
            </div>

            {/* Add Spec Modal */}
            {showAddSpec && (
                <div className="fixed inset-0 bg-black/80 flex items-center justify-center z-50">
                    <div className="bg-zinc-950 border border-zinc-800 w-full max-w-2xl max-h-[80vh] flex flex-col">
                        <div className="p-4 border-b border-zinc-900 flex items-center justify-between">
                            <h2 className="text-lg font-light">Add Specification</h2>
                            <button
                                onClick={() => setShowAddSpec(false)}
                                className="text-zinc-500 hover:text-white transition-colors"
                            >
                                Close
                            </button>
                        </div>
                        <div className="flex-1 p-4 overflow-hidden">
                            <textarea
                                value={newSpecContent}
                                onChange={(e) => setNewSpecContent(e.target.value)}
                                placeholder={`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";

contract MySpec is Test {
    function setUp() public {
        // Setup
    }

    function invariant_example() public {
        // Your invariant here
    }
}`}
                                className="w-full h-64 p-4 bg-black border border-zinc-800 text-zinc-300 font-mono text-sm resize-none focus:outline-none focus:border-zinc-700"
                                spellCheck={false}
                            />
                        </div>
                        <div className="p-4 border-t border-zinc-900 flex justify-end gap-4">
                            <button
                                onClick={() => setShowAddSpec(false)}
                                className="px-6 py-2 text-sm text-zinc-400 hover:text-white transition-colors"
                            >
                                Cancel
                            </button>
                            <button
                                onClick={handleAddSpec}
                                disabled={!newSpecContent.trim()}
                                className={`px-6 py-2 text-sm font-medium transition-colors ${newSpecContent.trim()
                                    ? 'bg-white text-black hover:bg-zinc-200'
                                    : 'bg-zinc-800 text-zinc-600 cursor-not-allowed'
                                    }`}
                            >
                                Add Specification
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}
