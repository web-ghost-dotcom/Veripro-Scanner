# VeriPro Smart Contracts

On-chain attestation registry for VeriPro verification results.

## Overview

The VeriPro smart contracts provide an immutable, on-chain record of formal verification results. When CBSE verifies a smart contract, it produces a signed attestation that can be committed to the registry, creating a permanent proof of verification.

## Contracts

### AttestationRegistry

The main contract that stores verification attestations.

**Features:**
- Stores verification results on-chain
- Signature verification from authorized provers
- Prover authorization management
- Event emission for indexing

**Key Functions:**

```solidity
// Commit a verification attestation (anyone can call, signature must be from authorized prover)
function commitAttestation(
    bytes32 resultHash,    // Hash of the verification result
    bool passed,           // Whether verification passed
    bytes32 contractHash,  // Hash of the verified bytecode
    uint8 v, bytes32 r, bytes32 s  // ECDSA signature
) external;

// Authorize or revoke a prover (owner only)
function setProver(address prover, bool status) external;

// Check if an address is an authorized prover
function isProver(address) external view returns (bool);
```

**Events:**

```solidity
event VerificationAttested(
    bytes32 indexed resultHash,
    address indexed prover,
    bool passed,
    bytes32 contractHash
);

event ProverAuthorized(address prover);
event ProverRevoked(address prover);
```

## Prerequisites

- [Foundry](https://book.getfoundry.sh/getting-started/installation)

## Installation

```bash
# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Install dependencies
forge install
```

## Build

```bash
forge build
```

## Test

```bash
# Run all tests
forge test

# Run with verbosity
forge test -vvv

# Run specific test
forge test --match-test testDeposit
```

## Deployment

### Local (Anvil)

```bash
# Terminal 1: Start local node
anvil

# Terminal 2: Deploy
forge script script/DeployRegistry.s.sol:DeployRegistry \
    --rpc-url http://127.0.0.1:8545 \
    --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
    --broadcast
```

### Deployment

```bash
export PRIVATE_KEY=your_deployer_private_key
export RPC_URL=https://eth-sepolia.public.blastapi.io

forge script script/DeployRegistry.s.sol:DeployRegistry \
    --rpc-url $RPC_URL \
    --private-key $PRIVATE_KEY \
    --broadcast \
    --verify
```

## Post-Deployment

### Authorize Provers

After deployment, authorize your CBSE coordinator's prover address:

```bash
# Using cast
cast send $REGISTRY_ADDRESS "setProver(address,bool)" $PROVER_ADDRESS true \
    --rpc-url $RPC_URL \
    --private-key $OWNER_PRIVATE_KEY
```

### Verify Prover Status

```bash
cast call $REGISTRY_ADDRESS "isProver(address)" $PROVER_ADDRESS \
    --rpc-url $RPC_URL
```

## Usage

### Committing Attestations

Attestations are typically committed by the VeriPro frontend after verification. The flow:

1. User verifies a contract through VeriPro
2. CBSE coordinator returns a signed attestation
3. Frontend calls `commitAttestation` with the signature
4. Registry verifies the signature and emits an event

### Querying Attestations

Attestations are stored as events. Query using:

```bash
# Using cast
cast logs --address $REGISTRY_ADDRESS \
    --from-block 0 \
    "VerificationAttested(bytes32,address,bool,bytes32)"
```

Or use an indexer like The Graph to query historical attestations.

## Project Structure

```
smart-contracts/
├── src/
│   ├── AttestationRegistry.sol  # Main registry contract
│   └── Counter.sol              # Example contract
├── script/
│   ├── DeployRegistry.s.sol     # Deployment script
│   └── Counter.s.sol            # Example script
├── test/
│   └── Counter.t.sol            # Example tests
├── lib/
│   └── forge-std/               # Foundry standard library
└── foundry.toml                 # Foundry configuration
```

## Security Considerations

- Only the owner can authorize/revoke provers
- Attestations require valid signatures from authorized provers
- The registry does not store the actual verification data, only hashes
- Anyone can call `commitAttestation`, but signature verification ensures authenticity

## Network Information

### Supported Chains

- Ethereum Mainnet
- Sepolia Testnet
- Any EVM-compatible chain

## Gas Costs

| Function | Approximate Gas |
|----------|----------------|
| `commitAttestation` | ~45,000 |
| `setProver` | ~25,000 |

## License

MIT
