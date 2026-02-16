'use client';

// AttestationRegistry contract address
// For production, use environment variable to override
export const ATTESTATION_REGISTRY_ADDRESS = process.env.NEXT_PUBLIC_ATTESTATION_REGISTRY_ADDRESS || '0xae454F272197b110C28223dbE3e49b4a1c798015';

// AttestationRegistry ABI (only the functions we need)
export const ATTESTATION_REGISTRY_ABI = [
    {
        "inputs": [
            { "internalType": "bytes32", "name": "resultHash", "type": "bytes32" },
            { "internalType": "bool", "name": "passed", "type": "bool" },
            { "internalType": "bytes32", "name": "contractHash", "type": "bytes32" },
            { "internalType": "uint8", "name": "v", "type": "uint8" },
            { "internalType": "bytes32", "name": "r", "type": "bytes32" },
            { "internalType": "bytes32", "name": "s", "type": "bytes32" }
        ],
        "name": "commitAttestation",
        "outputs": [],
        "stateMutability": "nonpayable",
        "type": "function"
    },
    {
        "inputs": [{ "internalType": "address", "name": "", "type": "address" }],
        "name": "isProver",
        "outputs": [{ "internalType": "bool", "name": "", "type": "bool" }],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "owner",
        "outputs": [{ "internalType": "address", "name": "", "type": "address" }],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "anonymous": false,
        "inputs": [
            { "indexed": true, "internalType": "bytes32", "name": "resultHash", "type": "bytes32" },
            { "indexed": true, "internalType": "address", "name": "prover", "type": "address" },
            { "indexed": false, "internalType": "bool", "name": "passed", "type": "bool" },
            { "indexed": false, "internalType": "bytes32", "name": "contractHash", "type": "bytes32" }
        ],
        "name": "VerificationAttested",
        "type": "event"
    }
] as const;

// Helper to parse signature from hex string to v, r, s
export function parseSignature(signatureHex: string): { v: number; r: `0x${string}`; s: `0x${string}` } | null {
    try {
        // Remove 0x prefix if present
        const sig = signatureHex.startsWith('0x') ? signatureHex.slice(2) : signatureHex;

        // Handle both 64-byte (128 chars, no v) and 65-byte (130 chars, with v) signatures
        if (sig.length !== 128 && sig.length !== 130) {
            console.error('Invalid signature length:', sig.length, '(expected 128 or 130)');
            return null;
        }

        const r = `0x${sig.slice(0, 64)}` as `0x${string}`;
        const s = `0x${sig.slice(64, 128)}` as `0x${string}`;
        
        let v: number;
        if (sig.length === 130) {
            // 65-byte signature includes v
            v = parseInt(sig.slice(128, 130), 16);
        } else {
            // 64-byte signature - need to try both v values (27 and 28)
            // Default to 27, the contract's ecrecover will validate
            v = 27;
        }

        // Handle EIP-155 signature format
        if (v < 27) {
            v += 27;
        }

        return { v, r, s };
    } catch (err) {
        console.error('Failed to parse signature:', err);
        return null;
    }
}

// Convert string to bytes32
export function stringToBytes32(str: string): `0x${string}` {
    // If already a hex string, pad/truncate to 32 bytes
    if (str.startsWith('0x')) {
        const hex = str.slice(2).padEnd(64, '0').slice(0, 64);
        return `0x${hex}`;
    }
    // Otherwise, hash the string
    // Simple approach: pad the string as hex
    const hex = Buffer.from(str).toString('hex').padEnd(64, '0').slice(0, 64);
    return `0x${hex}`;
}
