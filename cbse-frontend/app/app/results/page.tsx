'use client';

import { useState, useEffect } from 'react';
import { useAuth } from '../layout';
import { useAccount } from 'wagmi';
import { getResults, deleteResult, clearAllResults, type VerificationResult } from '../../lib/store';
import { ConnectButton } from '@rainbow-me/rainbowkit';

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

export default function ResultsPage() {
    const { chain } = useAccount();
    const { isConnected } = useAuth();
    const [results, setResults] = useState<VerificationResult[]>([]);
    const [selectedResult, setSelectedResult] = useState<VerificationResult | null>(null);
    const [filter, setFilter] = useState<'all' | 'verified' | 'failed'>('all');

    useEffect(() => {
        if (isConnected) {
            // eslint-disable-next-line react-hooks/set-state-in-effect
            setResults(getResults());
        }
    }, [isConnected]);

    const filteredResults = results.filter(r => {
        if (filter === 'all') return true;
        return r.status === filter;
    });

    if (!isConnected) {
        return (
            <div className="flex items-center justify-center min-h-[calc(100vh-64px)]">
                <div className="text-center max-w-md">
                    <h1 className="text-3xl font-light mb-4">Verification Results</h1>
                    <p className="text-zinc-500 mb-8">
                        Connect your wallet to view your verification history.
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
            <div className="px-6 py-4 border-b border-zinc-900">
                <div className="flex items-center justify-between mb-4">
                    <div>
                        <h1 className="text-xl font-light">Verification Results</h1>
                        <p className="text-sm text-zinc-500 mt-1">
                            View and manage your verification history
                        </p>
                    </div>
                    <div className="flex gap-2">
                        <button
                            onClick={() => {
                                if (results.length === 0) return;
                                const exportData = results.map(r => ({
                                    contractName: r.contractName,
                                    status: r.status,
                                    timestamp: r.timestamp,
                                    bytecodeHash: r.bytecodeHash,
                                    specHash: r.specHash,
                                    signature: r.signature,
                                    attestation: r.attestation,
                                    onchainTxHash: r.onchainTxHash,
                                }));
                                const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' });
                                const url = URL.createObjectURL(blob);
                                const a = document.createElement('a');
                                a.href = url;
                                a.download = `veripro-results-${new Date().toISOString().split('T')[0]}.json`;
                                a.click();
                                URL.revokeObjectURL(url);
                            }}
                            disabled={results.length === 0}
                            className="px-4 py-2 text-sm bg-zinc-900 text-zinc-400 hover:text-white disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        >
                            Export All
                        </button>
                        <button
                            onClick={() => {
                                if (confirm('Clear all verification history? This cannot be undone.')) {
                                    clearAllResults();
                                    setResults([]);
                                    setSelectedResult(null);
                                }
                            }}
                            disabled={results.length === 0}
                            className="px-4 py-2 text-sm bg-zinc-900 text-red-400 hover:text-red-300 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        >
                            Clear History
                        </button>
                    </div>
                </div>

                {/* Filters */}
                <div className="flex gap-2">
                    {(['all', 'verified', 'failed'] as const).map((f) => (
                        <button
                            key={f}
                            onClick={() => setFilter(f)}
                            className={`px-3 py-1 text-sm transition-colors ${filter === f
                                ? 'bg-white text-black'
                                : 'bg-zinc-900 text-zinc-400 hover:text-white'
                                }`}
                        >
                            {f.charAt(0).toUpperCase() + f.slice(1)}
                        </button>
                    ))}
                </div>
            </div>

            {/* Content */}
            <div className="flex-1 flex overflow-hidden">
                {/* Results List */}
                <div className="w-full lg:w-1/2 border-r border-zinc-900 overflow-y-auto">
                    {filteredResults.length === 0 ? (
                        <div className="p-8 text-center text-zinc-500">
                            No results found
                        </div>
                    ) : (
                        <div className="divide-y divide-zinc-900">
                            {filteredResults.map((result) => (
                                <button
                                    key={result.id}
                                    onClick={() => setSelectedResult(result)}
                                    className={`w-full p-4 text-left transition-colors ${selectedResult?.id === result.id
                                        ? 'bg-zinc-900'
                                        : 'hover:bg-zinc-900/50'
                                        }`}
                                >
                                    <div className="flex items-center justify-between mb-2">
                                        <span className="font-medium">{result.contractName}</span>
                                        <span className="text-xs text-zinc-500">
                                            {formatRelativeTime(result.timestamp)}
                                        </span>
                                    </div>
                                    <div className="flex items-center gap-2">
                                        <span className={`text-xs px-2 py-0.5 ${result.status === 'verified'
                                            ? 'bg-green-900/30 text-green-400'
                                            : 'bg-red-900/30 text-red-400'
                                            }`}>
                                            {result.status}
                                        </span>
                                        <span className="text-xs text-zinc-600">
                                            {result.properties.length} properties checked
                                        </span>
                                    </div>
                                </button>
                            ))}
                        </div>
                    )}
                </div>

                {/* Detail Panel */}
                <div className="hidden lg:block flex-1 overflow-y-auto">
                    {!selectedResult ? (
                        <div className="flex items-center justify-center h-full text-zinc-600">
                            Select a result to view details
                        </div>
                    ) : (
                        <div className="p-6 space-y-6">
                            {/* Header */}
                            <div>
                                <div className="flex items-center gap-3 mb-2">
                                    <h2 className="text-xl font-light">{selectedResult.contractName}</h2>
                                    <span className={`text-xs px-2 py-0.5 ${selectedResult.status === 'verified'
                                        ? 'bg-green-900/30 text-green-400'
                                        : 'bg-red-900/30 text-red-400'
                                        }`}>
                                        {selectedResult.status}
                                    </span>
                                </div>
                                <p className="text-sm text-zinc-500">
                                    {new Date(selectedResult.timestamp).toLocaleString()}
                                </p>
                            </div>

                            {/* Properties */}
                            <div>
                                <h3 className="text-xs text-zinc-500 uppercase tracking-wide mb-3">
                                    Properties
                                </h3>
                                <div className="space-y-2">
                                    {selectedResult.properties.map((prop, i) => (
                                        <div
                                            key={i}
                                            className="flex items-center justify-between p-3 bg-zinc-900/50"
                                        >
                                            <span className="text-sm text-zinc-300">{prop.name}</span>
                                            <span className={`text-xs ${prop.status === 'passed'
                                                ? 'text-green-400'
                                                : 'text-red-400'
                                                }`}>
                                                {prop.status}
                                            </span>
                                        </div>
                                    ))}
                                </div>
                            </div>

                            {/* Hashes */}
                            <div className="space-y-4">
                                <div>
                                    <h3 className="text-xs text-zinc-500 uppercase tracking-wide mb-2">
                                        Bytecode Hash
                                    </h3>
                                    <div className="p-3 bg-zinc-900/50 font-mono text-xs text-zinc-400 break-all">
                                        {selectedResult.bytecodeHash}
                                    </div>
                                </div>
                                <div>
                                    <h3 className="text-xs text-zinc-500 uppercase tracking-wide mb-2">
                                        Spec Hash
                                    </h3>
                                    <div className="p-3 bg-zinc-900/50 font-mono text-xs text-zinc-400 break-all">
                                        {selectedResult.specHash}
                                    </div>
                                </div>
                                {selectedResult.signature && (
                                    <div>
                                        <h3 className="text-xs text-zinc-500 uppercase tracking-wide mb-2">
                                            Attestation Signature
                                        </h3>
                                        <div className="p-3 bg-zinc-900/50 font-mono text-xs text-zinc-400 break-all">
                                            {selectedResult.signature}
                                        </div>
                                    </div>
                                )}
                            </div>

                            {/* On-chain attestation info */}
                            {selectedResult.onchainAttested && selectedResult.onchainTxHash && (
                                <div className="p-4 bg-green-900/20 border border-green-900/50 rounded">
                                    <div className="flex items-center gap-2 text-green-400 text-sm mb-2">
                                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                                        </svg>
                                        Attested On-Chain
                                    </div>
                                    <a
                                        href={`${chain?.blockExplorers?.default.url || 'https://etherscan.io'}/tx/${selectedResult.onchainTxHash}`}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        className="text-xs text-zinc-400 hover:text-zinc-300 font-mono break-all"
                                    >
                                        {selectedResult.onchainTxHash}
                                    </a>
                                </div>
                            )}

                            {/* Actions */}
                            <div className="flex flex-wrap gap-3 pt-4 border-t border-zinc-800">
                                <button
                                    onClick={() => {
                                        navigator.clipboard.writeText(JSON.stringify({
                                            contractName: selectedResult.contractName,
                                            bytecodeHash: selectedResult.bytecodeHash,
                                            specHash: selectedResult.specHash,
                                            signature: selectedResult.signature,
                                            attestation: selectedResult.attestation,
                                        }, null, 2));
                                    }}
                                    className="px-4 py-2 text-sm bg-zinc-900 text-zinc-300 hover:bg-zinc-800 transition-colors"
                                >
                                    Copy Attestation
                                </button>
                                <button
                                    onClick={() => {
                                        const exportData = {
                                            contractName: selectedResult.contractName,
                                            status: selectedResult.status,
                                            timestamp: selectedResult.timestamp,
                                            bytecodeHash: selectedResult.bytecodeHash,
                                            specHash: selectedResult.specHash,
                                            signature: selectedResult.signature,
                                            attestation: selectedResult.attestation,
                                            onchainTxHash: selectedResult.onchainTxHash,
                                        };
                                        const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' });
                                        const url = URL.createObjectURL(blob);
                                        const a = document.createElement('a');
                                        a.href = url;
                                        a.download = `${selectedResult.contractName}-attestation.json`;
                                        a.click();
                                        URL.revokeObjectURL(url);
                                    }}
                                    className="px-4 py-2 text-sm bg-zinc-900 text-zinc-300 hover:bg-zinc-800 transition-colors"
                                >
                                    Export JSON
                                </button>
                                <button
                                    onClick={() => {
                                        if (confirm(`Delete this verification result?`)) {
                                            deleteResult(selectedResult.id);
                                            setResults(getResults());
                                            setSelectedResult(null);
                                        }
                                    }}
                                    className="px-4 py-2 text-sm bg-zinc-900 text-red-400 hover:text-red-300 transition-colors"
                                >
                                    Delete
                                </button>
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
