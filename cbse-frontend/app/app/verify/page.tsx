'use client';

import { useState } from 'react';
import Link from 'next/link';
import { useAuth } from '../layout';
import { addResult } from '../../lib/store';
import { ConnectButton } from '@rainbow-me/rainbowkit';

export default function QuickVerifyPage() {
    const { isConnected } = useAuth();
    const [contractSource, setContractSource] = useState('');
    const [specSource, setSpecSource] = useState('');
    const [contractName, setContractName] = useState('');
    const [isVerifying, setIsVerifying] = useState(false);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const [result, setResult] = useState<any>(null);
    const [activePanel, setActivePanel] = useState<'contract' | 'spec'>('contract');

    const handleVerify = async () => {
        if (!contractSource.trim() || !specSource.trim()) {
            setResult({ status: 'Error', message: 'Both contract and specification are required' });
            return;
        }

        setIsVerifying(true);
        setResult(null);

        try {
            const res = await fetch('/api/verify', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    contract_source: contractSource,
                    spec_source: specSource,
                    contract_name: contractName || 'Contract'
                })
            });

            if (res.ok) {
                const data = await res.json();
                setResult(data);

                // Save result to store (quick verify has no project)
                addResult({
                    contractName: contractName || 'Contract',
                    status: data.status === 'Success' ? 'verified' : 'failed',
                    message: data.message,
                    bytecodeHash: data.attestation?.bytecode_hash,
                    specHash: data.attestation?.spec_hash,
                    signature: data.attestation?.signature,
                    properties: [],
                    attestation: data.attestation,
                });
            } else {
                const errText = await res.text();
                setResult({ status: 'Error', message: errText || 'Verification failed' });
            }
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            setResult({ status: 'Error', message: message || 'Connection failed' });
        } finally {
            setIsVerifying(false);
        }
    };

    if (!isConnected) {
        return (
            <div className="flex items-center justify-center min-h-[calc(100vh-64px)]">
                <div className="text-center max-w-md">
                    <h1 className="text-3xl font-light mb-4">Quick Verify</h1>
                    <p className="text-zinc-500 mb-8">
                        Connect your wallet to access the verification tools.
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
                <div>
                    <h1 className="text-xl font-light">Quick Verify</h1>
                    <p className="text-sm text-zinc-500 mt-1">
                        Paste contract and specification code for immediate verification
                    </p>
                </div>
                <div className="flex items-center gap-4">
                    <input
                        type="text"
                        value={contractName}
                        onChange={(e) => setContractName(e.target.value)}
                        placeholder="Contract name"
                        className="px-4 py-2 bg-zinc-900 border border-zinc-800 text-white text-sm focus:outline-none focus:border-zinc-700"
                    />
                    <button
                        onClick={handleVerify}
                        disabled={isVerifying}
                        className={`px-6 py-2 text-sm font-medium transition-colors ${isVerifying
                            ? 'bg-zinc-800 text-zinc-500 cursor-not-allowed'
                            : 'bg-white text-black hover:bg-zinc-200'
                            }`}
                    >
                        {isVerifying ? 'Verifying...' : 'Verify'}
                    </button>
                </div>
            </div>

            {/* Main Content */}
            <div className="flex-1 flex overflow-hidden">
                {/* Editor Panels */}
                <div className="flex-1 flex flex-col lg:flex-row overflow-hidden">
                    {/* Mobile Tabs */}
                    <div className="lg:hidden flex border-b border-zinc-900">
                        <button
                            onClick={() => setActivePanel('contract')}
                            className={`flex-1 px-4 py-3 text-sm transition-colors ${activePanel === 'contract'
                                ? 'bg-zinc-900 text-white'
                                : 'text-zinc-500'
                                }`}
                        >
                            Contract
                        </button>
                        <button
                            onClick={() => setActivePanel('spec')}
                            className={`flex-1 px-4 py-3 text-sm transition-colors ${activePanel === 'spec'
                                ? 'bg-zinc-900 text-white'
                                : 'text-zinc-500'
                                }`}
                        >
                            Specification
                        </button>
                    </div>

                    {/* Contract Panel */}
                    <div className={`flex-1 flex flex-col border-r border-zinc-900 ${activePanel !== 'contract' ? 'hidden lg:flex' : ''
                        }`}>
                        <div className="px-4 py-3 border-b border-zinc-900 text-xs text-zinc-500 uppercase tracking-wide">
                            Contract Source
                        </div>
                        <textarea
                            value={contractSource}
                            onChange={(e) => setContractSource(e.target.value)}
                            placeholder="// Paste your Solidity contract here..."
                            className="flex-1 p-4 bg-black text-zinc-300 font-mono text-sm resize-none focus:outline-none placeholder-zinc-700"
                            spellCheck={false}
                        />
                    </div>

                    {/* Spec Panel */}
                    <div className={`flex-1 flex flex-col ${activePanel !== 'spec' ? 'hidden lg:flex' : ''
                        }`}>
                        <div className="px-4 py-3 border-b border-zinc-900 text-xs text-zinc-500 uppercase tracking-wide">
                            Specification
                        </div>
                        <textarea
                            value={specSource}
                            onChange={(e) => setSpecSource(e.target.value)}
                            placeholder="// Paste your specification here..."
                            className="flex-1 p-4 bg-black text-zinc-300 font-mono text-sm resize-none focus:outline-none placeholder-zinc-700"
                            spellCheck={false}
                        />
                    </div>
                </div>

                {/* Results Panel */}
                <div className="w-full lg:w-96 border-t lg:border-t-0 lg:border-l border-zinc-900 flex flex-col">
                    <div className="px-4 py-3 border-b border-zinc-900 text-xs text-zinc-500 uppercase tracking-wide">
                        Results
                    </div>
                    <div className="flex-1 overflow-auto p-4">
                        {!result && !isVerifying && (
                            <div className="text-center text-zinc-600 text-sm py-12">
                                <p className="mb-4">No verification results yet</p>
                                <p className="text-xs text-zinc-700">
                                    Paste your contract and specification, then click Verify
                                </p>
                            </div>
                        )}

                        {isVerifying && (
                            <div className="text-center py-12">
                                <div className="w-8 h-8 border-2 border-white border-t-transparent rounded-full animate-spin mx-auto mb-4" />
                                <p className="text-sm text-zinc-500">Running verification...</p>
                                <p className="text-xs text-zinc-600 mt-2">
                                    This may take a few moments
                                </p>
                            </div>
                        )}

                        {result && (
                            <div className="space-y-4">
                                {/* Status */}
                                <div className={`p-4 ${result.status === 'Success'
                                    ? 'bg-green-900/20 border border-green-900/50'
                                    : result.status === 'Error'
                                        ? 'bg-red-900/20 border border-red-900/50'
                                        : 'bg-zinc-900/50 border border-zinc-800'
                                    }`}>
                                    <div className={`text-sm font-medium mb-2 ${result.status === 'Success'
                                        ? 'text-green-400'
                                        : result.status === 'Error'
                                            ? 'text-red-400'
                                            : 'text-zinc-400'
                                        }`}>
                                        {result.status}
                                    </div>
                                    <p className="text-xs text-zinc-400">
                                        {result.message}
                                    </p>
                                </div>

                                {/* Attestation */}
                                {result.attestation && (
                                    <div className="space-y-3">
                                        <div className="p-4 bg-zinc-900/50 border border-zinc-800">
                                            <div className="text-xs text-zinc-500 uppercase mb-2">Contract</div>
                                            <div className="text-sm text-zinc-300">{result.attestation.contract_name}</div>
                                        </div>

                                        <div className="p-4 bg-zinc-900/50 border border-zinc-800">
                                            <div className="text-xs text-zinc-500 uppercase mb-2">Bytecode Hash</div>
                                            <div className="text-xs font-mono text-zinc-400 break-all">
                                                {result.attestation.bytecode_hash}
                                            </div>
                                        </div>

                                        <div className="p-4 bg-zinc-900/50 border border-zinc-800">
                                            <div className="text-xs text-zinc-500 uppercase mb-2">Spec Hash</div>
                                            <div className="text-xs font-mono text-zinc-400 break-all">
                                                {result.attestation.spec_hash}
                                            </div>
                                        </div>

                                        {result.attestation.signature && (
                                            <div className="p-4 bg-zinc-900/50 border border-zinc-800">
                                                <div className="text-xs text-zinc-500 uppercase mb-2">Signature</div>
                                                <div className="text-xs font-mono text-zinc-400 break-all">
                                                    {result.attestation.signature}
                                                </div>
                                            </div>
                                        )}
                                    </div>
                                )}

                                {/* Actions */}
                                <div className="pt-4 border-t border-zinc-800 space-y-2">
                                    <button
                                        onClick={() => {
                                            if (result.attestation) {
                                                navigator.clipboard.writeText(JSON.stringify(result.attestation, null, 2));
                                            }
                                        }}
                                        className="w-full px-4 py-2 text-sm bg-zinc-900 text-zinc-300 hover:bg-zinc-800 transition-colors"
                                    >
                                        Copy Attestation
                                    </button>
                                    <Link
                                        href="/app/results"
                                        className="block w-full px-4 py-2 text-sm text-center text-zinc-500 hover:text-zinc-300 transition-colors"
                                    >
                                        View All Results
                                    </Link>
                                </div>
                            </div>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
