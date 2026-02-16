'use client';

import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { createWalletClient, custom, parseApi } from 'viem';
import 'viem/window';

const ATTESTATION_REGISTRY_ABI = [
    {
        "type": "function",
        "name": "commitAttestation",
        "inputs": [
            { "name": "resultHash", "type": "bytes32" },
            { "name": "passed", "type": "bool" },
            { "name": "contractHash", "type": "bytes32" },
            { "name": "v", "type": "uint8" },
            { "name": "r", "type": "bytes32" },
            { "name": "s", "type": "bytes32" }
        ],
        "outputs": [],
        "stateMutability": "nonpayable"
    }
];

const DEFAULT_CONTRACT = `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
contract Vault {
    uint256 public balance;
    function deposit() external payable {
        balance += msg.value;
    }
}`;

const DEFAULT_SPEC = `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
import "forge-std/Test.sol";
import "./Vault.sol";

contract VaultTest is Test {
    Vault vault;
    function setUp() public {
        vault = new Vault();
    }
    function invariant_balance_solvency() public {
        assertEq(address(vault).balance, vault.balance());
    }
}`;

interface VerificationResponse {
    status: string;
    verdict?: string;
    output?: string;
    job_id?: string;
    message?: string;
    attestation?: {
        prover_address: string;
        result_hash: string;
        signature: string;
        payload?: {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            details?: any;
            [key: string]: unknown;
        };
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

export default function DemoInterface() {
    const [contractSource, setContractSource] = useState(DEFAULT_CONTRACT);
    const [specSource, setSpecSource] = useState(DEFAULT_SPEC);
    const [isLoading, setIsLoading] = useState(false);
    const [result, setResult] = useState<VerificationResponse | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [isPublishing, setIsPublishing] = useState(false);
    const [txHash, setTxHash] = useState<string | null>(null);
    const [publishError, setPublishError] = useState<string | null>(null);

    const handlePublish = async () => {
        if (!result?.attestation) return;
        setIsPublishing(true);
        setPublishError(null);
        setTxHash(null);

        try {
            if (!window.ethereum) {
                throw new Error("No crypto wallet found. Please install MetaMask.");
            }

            const client = createWalletClient({
                transport: custom(window.ethereum as any)
            });

            const [account] = await client.requestAddresses();

            // Parse signature
            const signature = result.attestation.signature.startsWith('0x')
                ? result.attestation.signature
                : `0x${result.attestation.signature}`;

            // simple parsing of r,s,v from 65-byte signature
            const r = `0x${signature.slice(2, 66)}` as `0x${string}`;
            const s = `0x${signature.slice(66, 130)}` as `0x${string}`;
            let v = parseInt(signature.slice(130, 132), 16);

            // Adjust v for Ethereum (27/28) if it's 0/1
            if (v < 27) v += 27;

            const registryAddress = process.env.NEXT_PUBLIC_ATTESTATION_REGISTRY_ADDRESS as `0x${string}`;
            if (!registryAddress) throw new Error("Registry address not configured");

            // Extract contract hash from payload - safe cast since we know structure
            const payload = result.attestation.payload as any || {};
            const contractHash = payload.contract_bytecode_hash as `0x${string}`
                || '0x0000000000000000000000000000000000000000000000000000000000000000';

            const resultHash = (result.attestation.result_hash.startsWith('0x')
                ? result.attestation.result_hash
                : `0x${result.attestation.result_hash}`) as `0x${string}`;

            const passed = payload.passed === true;

            const hash = await client.writeContract({
                address: registryAddress,
                abi: ATTESTATION_REGISTRY_ABI,
                functionName: 'commitAttestation',
                args: [resultHash, passed, contractHash, v, r, s],
                account
            });

            setTxHash(hash);

        } catch (err: any) {
            console.error(err);
            setPublishError(err.message || "Failed to publish attestation");
        } finally {
            setIsPublishing(false);
        }
    };

    const handleVerify = async () => {
        setIsLoading(true);
        setResult(null);
        setError(null);

        try {
            const res = await fetch('/api/verify', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    contract_source: contractSource,
                    spec_source: specSource,
                    contract_name: "Vault"
                })
            });

            if (!res.ok) {
                const text = await res.text();
                try {
                    // try to parse json error if possible
                    const jsonErr = JSON.parse(text);
                    throw new Error(jsonErr.message || `Server error ${res.status}`);
                } catch {
                    throw new Error(text || `Server returned ${res.status}`);
                }
            }

            const data = await res.json();
            setResult(data);
        } catch (error) {
            const message = error instanceof Error ? error.message : 'Analysis failed';
            setError(message);
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <div className="w-full max-w-7xl mx-auto px-6 py-12" id="demo">
            <div className="mb-12 text-center">
                <h2 className="text-3xl font-light mb-4">Live Demo</h2>
                <p className="text-zinc-500">Run the example verification locally to see VeriPro in action.</p>
            </div>

            <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                className="grid grid-cols-1 lg:grid-cols-2 gap-8"
            >
                {/* Inputs */}
                <div className="space-y-6">
                    <div>
                        <label className="block text-zinc-400 text-xs uppercase tracking-wider mb-2">Contract Code</label>
                        <textarea
                            value={contractSource}
                            onChange={(e) => setContractSource(e.target.value)}
                            className="w-full h-64 bg-zinc-900/50 border border-zinc-800 rounded p-4 font-mono text-sm text-zinc-300 focus:outline-none focus:border-white transition-colors resize-none"
                            spellCheck={false}
                        />
                    </div>
                    <div>
                        <label className="block text-zinc-400 text-xs uppercase tracking-wider mb-2">Specification</label>
                        <textarea
                            value={specSource}
                            onChange={(e) => setSpecSource(e.target.value)}
                            className="w-full h-64 bg-zinc-900/50 border border-zinc-800 rounded p-4 font-mono text-sm text-zinc-300 focus:outline-none focus:border-white transition-colors resize-none"
                            spellCheck={false}
                        />
                    </div>

                    <motion.button
                        onClick={handleVerify}
                        disabled={isLoading}
                        whileHover={{ scale: 1.02 }}
                        whileTap={{ scale: 0.98 }}
                        animate={{
                            boxShadow: isLoading
                                ? "0px 0px 0px rgba(255, 255, 255, 0)"
                                : ["0px 0px 0px rgba(255, 255, 255, 0)", "0px 0px 20px rgba(255, 255, 255, 0.3)", "0px 0px 0px rgba(255, 255, 255, 0)"]
                        }}
                        transition={{
                            boxShadow: {
                                duration: 2,
                                repeat: Infinity,
                            }
                        }}
                        className={`w-full py-4 text-black font-medium tracking-wide transition-all rounded-sm relative overflow-hidden group
                            ${isLoading ? 'bg-zinc-500 cursor-not-allowed' : 'bg-white hover:bg-zinc-200'}
                        `}
                    >
                        <span className="relative z-10 flex items-center justify-center gap-2">
                            {isLoading ? 'Verifying...' : 'Run Verification Protocol'}
                            {!isLoading && (
                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                                    <path d="M5 12h14M12 5l7 7-7 7" />
                                </svg>
                            )}
                        </span>
                        {!isLoading && <div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/50 to-transparent -translate-x-full group-hover:animate-[shimmer_1s_infinite] w-full" />}
                    </motion.button>
                </div>


                {/* Output */}
                <div className="relative min-h-[500px] bg-black border border-zinc-800 rounded p-6 font-mono overflow-hidden flex flex-col">
                    <div className="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-transparent via-zinc-700 to-transparent opacity-20"></div>

                    {!result && !isLoading && !error && (
                        <div className="flex h-full items-center justify-center text-zinc-600">
                            Waiting for submission...
                        </div>
                    )}

                    {isLoading && (
                        <div className="flex flex-col h-full items-center justify-center gap-4">
                            <div className="w-12 h-12 border-2 border-white border-t-transparent rounded-full animate-spin"></div>
                            <div className="text-zinc-400 text-sm animate-pulse">Running Prover Node...</div>
                        </div>
                    )}

                    {error && (
                        <div className="text-red-500 p-4 border border-red-900/50 bg-red-900/10 rounded">
                            <div className="font-bold mb-2">Error</div>
                            {error}
                        </div>
                    )}

                    {result && (
                        <motion.div
                            initial={{ opacity: 0, y: 10 }}
                            animate={{ opacity: 1, y: 0 }}
                            className="space-y-4 text-sm h-full overflow-y-auto"
                        >
                            <div className="flex items-center gap-2 pb-4 border-b border-zinc-900">
                                <div className={`w-3 h-3 rounded-full ${result.status === 'Success' ? 'bg-green-500' : 'bg-red-500'}`}></div>
                                <span className={result.status === 'Success' ? 'text-green-500' : 'text-red-500'}>
                                    {result.status}
                                </span>
                                <span className="text-zinc-600 ml-auto text-xs">{result.job_id}</span>
                            </div>

                            <div className="text-zinc-300">
                                {result.message}
                            </div>

                            {/* Attestation Details */}
                            {result.attestation && (
                                <div className="mt-8 space-y-2">
                                    <div className="text-zinc-500 text-xs uppercase">Cryptographic Attestation</div>
                                    <div className="bg-zinc-900/50 p-4 rounded overflow-x-auto text-xs text-zinc-400">
                                        <div className="mb-2">
                                            <span className="text-zinc-600">Prover:</span> <span className="text-zinc-300">{result.attestation.prover_address}</span>
                                        </div>
                                        <div className="mb-2">
                                            <span className="text-zinc-600">Result Hash:</span> <span className="text-zinc-300">{result.attestation.result_hash}</span>
                                        </div>
                                        <div className="mb-2 break-all">
                                            <span className="text-zinc-600">Signature:</span><br />
                                            {result.attestation.signature}
                                        </div>
                                    </div>

                                    <div className="pt-4 border-t border-zinc-800">
                                        <button
                                            onClick={handlePublish}
                                            disabled={isPublishing || !!txHash}
                                            className="w-full py-2 bg-white text-black hover:bg-zinc-200 disabled:opacity-50 disabled:cursor-not-allowed text-xs font-bold uppercase tracking-wider rounded transition-colors"
                                        >
                                            {isPublishing ? 'Publishing to Registry...' : txHash ? 'Attestation Published' : 'Publish Attestation On-Chain'}
                                        </button>

                                        {publishError && (
                                            <div className="mt-2 text-red-500 text-xs text-center">{publishError}</div>
                                        )}

                                        {txHash && (
                                            <div className="mt-2 text-green-500 text-xs text-center break-all">
                                                TX: {txHash}
                                            </div>
                                        )}
                                    </div>

                                    {result.attestation.payload?.details && (
                                        <div className="mt-4 p-4 border border-zinc-800 rounded">
                                            <div className="text-zinc-500 text-xs uppercase mb-2">Analysis Trace</div>
                                            <pre className="text-xs text-green-400 overflow-x-auto whitespace-pre-wrap">
                                                {JSON.stringify(JSON.parse(result.attestation.payload.details), null, 2)}
                                            </pre>
                                        </div>
                                    )}
                                </div>
                            )}

                        </motion.div>
                    )}
                </div>
            </motion.div>
        </div>
    );
}
