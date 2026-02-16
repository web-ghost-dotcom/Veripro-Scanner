'use client';

import { useWriteContract, useWaitForTransactionReceipt, useReadContract } from 'wagmi';
import { ATTESTATION_REGISTRY_ADDRESS, ATTESTATION_REGISTRY_ABI, parseSignature } from './contracts';

export interface AttestationData {
    resultHash: string;
    passed: boolean;
    contractHash: string;
    signature: string;
}

export function useCommitAttestation() {
    const { writeContract, data: hash, isPending, error } = useWriteContract();

    const { isLoading: isConfirming, isSuccess } = useWaitForTransactionReceipt({
        hash,
    });

    const commitAttestation = async (attestation: AttestationData) => {
        const sig = parseSignature(attestation.signature);
        if (!sig) {
            throw new Error('Invalid signature format');
        }

        // Ensure hashes are properly formatted as bytes32
        const resultHash = attestation.resultHash.startsWith('0x')
            ? attestation.resultHash as `0x${string}`
            : `0x${attestation.resultHash}` as `0x${string}`;

        const contractHash = attestation.contractHash.startsWith('0x')
            ? attestation.contractHash as `0x${string}`
            : `0x${attestation.contractHash}` as `0x${string}`;

        writeContract({
            address: ATTESTATION_REGISTRY_ADDRESS as `0x${string}`,
            abi: ATTESTATION_REGISTRY_ABI,
            functionName: 'commitAttestation',
            args: [resultHash, attestation.passed, contractHash, sig.v, sig.r, sig.s],
        });
    };

    return {
        commitAttestation,
        hash,
        isPending,
        isConfirming,
        isSuccess,
        error,
    };
}

export function useIsProver(address?: string) {
    const { data, isLoading, error } = useReadContract({
        address: ATTESTATION_REGISTRY_ADDRESS as `0x${string}`,
        abi: ATTESTATION_REGISTRY_ABI,
        functionName: 'isProver',
        args: address ? [address as `0x${string}`] : undefined,
    });

    return {
        isProver: data as boolean | undefined,
        isLoading,
        error,
    };
}
