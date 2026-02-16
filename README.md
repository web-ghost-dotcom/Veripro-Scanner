# VeriPro

**AI-Agent Security Scanner & Formal Verification Platform**

VeriPro is a full-stack formal verification platform and AI Security Agent for smart contracts (EVM & BNB Chain). It combines a powerful symbolic execution engine (CBSE) with an autonomous AI agent enabling developers to:
1.  **Scan for Vulnerabilities**: AI agent identifies security risks (Reentrancy, Overflow, etc.).
2.  **Generate Specifications**: Automatically write formal proofs (invariants) for contracts.
3.  **Prove Correctness**: Use symbolic execution to mathematically verify properties.
4.  **Commit On-Chain**: Publish verification attestations to the blockchain.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           VERIPRO PLATFORM                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────┐   │
│   │  Frontend   │───▶│ Coordinator │───▶│  CBSE Symbolic Engine   │   │
│   │  (Next.js)  │    │   (Rust)    │    │  (Z3 SMT Solver)        │   │
│   └─────────────┘    └─────────────┘    └─────────────────────────┘   │
│         │                   │                       │                  │
│         │                   │                       │                  │
│         ▼                   ▼                       ▼                  │
│   ┌─────────────────────────────────────────────────────────────┐     │
│   │               Attestation Registry (EVM)                     │     │
│   │              On-chain verification proofs                    │     │
│   └─────────────────────────────────────────────────────────────┘     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## Repository Structure

```
VeriPro/
├── cbse-frontend/       # Next.js web application
├── FM-rust-cloud/       # CBSE symbolic execution engine + coordinator
├── smart-contracts/     # Attestation registry contracts
└── README.md           # This file
```

---

## Quick Start

### Prerequisites

- **Node.js** 18+ and npm
- **Rust** 1.70+ with Cargo
- **Z3 SMT Solver** 4.12+
- **Foundry** (forge, anvil, cast)

### 1. Clone the Repository

```bash
git clone https://github.com/your-org/veripro.git
cd veripro
```

### 2. Start All Services

```bash
# Terminal 1: Start the CBSE Coordinator
cd FM-rust-cloud
cargo build --release
cargo run --bin cbse-coordinator

# Terminal 2: Start the Frontend
cd cbse-frontend
npm install
npm run dev

# Terminal 3 (optional): Start local blockchain for testing
cd smart-contracts
anvil
```

### 3. Open the Platform

Navigate to [http://localhost:3000](http://localhost:3000) in your browser.

---

## Deployment

### Smart Contracts (BNB & EVM Chains)

Deploy the attestation registry to any EVM-compatible chain, including **BNB Chain**:

```bash
cd smart-contracts
# For BNB Chain Testnet
forge script script/DeployRegistry.s.sol:DeployRegistry \
  --rpc-url https://data-seed-prebsc-1-s1.binance.org:8545 \
  --broadcast \
  --verify

# For Sepolia
forge script script/DeployRegistry.s.sol:DeployRegistry \
  --rpc-url https://rpc.sepolia.org \
  --broadcast \
  --verify
```

### Coordinator (Rust)

Deploy the coordinator to a cloud server (AWS/GCP/DigitalOcean):

```bash
# Build for Linux
cargo build --release --target x86_64-unknown-linux-gnu
```

### Frontend (Vercel)

Deploy the frontend to Vercel:

1. Install Vercel CLI: `npm i -g vercel`
2. Run `vercel`

---

## Architecture

```
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│     Frontend    │       │   Coordinator   │       │   Blockchain    │
│    (Next.js)    │──────▶│     (Rust)      │──────▶│  (EVM Chain)    │
│                 │◀──────│                 │◀──────│                 │
└─────────────────┘       └─────────────────┘       └─────────────────┘
│     Vercel       │     │   Cloud Server   │     │    EVM Chain     │
```

---


# Create environment file
cp .env.example .env.local
```

#### Environment Variables

Create `.env.local`:

```env
# Google Gemini API (for AI spec generation)
GOOGLE_API_KEY=your_gemini_api_key

# Wallet Connect Project ID (optional)
NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID=your_project_id
```

#### Development

```bash
# Start development server
npm run dev

# Build for production
npm run build

# Start production server
npm start

# Run linting
npm run lint
```

#### Configuration

The frontend proxies verification requests to the coordinator. Configure in `next.config.ts`:

```typescript
const nextConfig: NextConfig = {
  async rewrites() {
    return [
      {
        source: '/api/verify',
        destination: 'http://127.0.0.1:3001/verify', // Coordinator URL
      },
    ];
  },
};
```

#### Deployment

**Vercel (Recommended):**
```bash
npm install -g vercel
vercel
```

**Docker:**
```dockerfile
FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build
EXPOSE 3000
CMD ["npm", "start"]
```

---

### CBSE Engine (`FM-rust-cloud/`)

The Complete Blockchain Symbolic Executor—a Rust-based symbolic execution engine for Ethereum smart contracts.

---

## AI Security Agent

VeriPro's "Agentic" system is designed to be more than just a chatbot. It acts as an autonomous security engineer:

1.  **Context-Aware Scanning**: The agent analyzes the full contract source code, identifying dependencies and inheritance structures.
2.  **Vulnerability Detection**: It proactively scans for common EVM and BNB Chain vulnerabilities (e.g., Reentrancy, Unchecked Return Values, Precision Loss).
3.  **Formal Spec Generation**: Instead of just pointing out errors, the agent **writes the code** to prove them. It generates Foundry invariant tests (`foundry-std`) that the CBSE engine uses to mathematically verify the contract.

**Agent Capabilities:**
*   `Scan`: fast vulnerability assessment.
*   `Audit`: deep-dive logic review.
*   `Prove`: generate mathematical proofs for specific properties.

---

## Development

### Prerequisites

- Complete EVM opcode support
- Z3-based symbolic constraint solving
- Path exploration with configurable bounds
- Foundry test integration
- Signed verification attestations
- HTTP coordinator for API access

#### Installation

```bash
cd FM-rust-cloud

# Install Z3 (required)
# macOS:
brew install z3

# Ubuntu/Debian:
sudo apt-get install z3 libz3-dev

# Build the project
cargo build --release
```

#### Binaries

The workspace produces two main binaries:

1. **`cbse`** - CLI symbolic executor
2. **`cbse-coordinator`** - HTTP API server

#### Running the Coordinator

```bash
# Development
cargo run --bin cbse-coordinator

# Production
./target/release/cbse-coordinator
```

The coordinator listens on `http://127.0.0.1:3001` by default.

#### Environment Variables

```bash
# Path to forge-std library (optional, auto-detected)
export FORGE_STD_PATH=/path/to/forge-std

# Prover private key for signing attestations
export PROVER_PRIVATE_KEY=your_private_key_hex

# Custom CBSE binary path
export CBSE_BINARY=/path/to/cbse
```

#### CLI Usage

```bash
# Run symbolic tests on a Foundry project
./target/release/cbse --root /path/to/project --match-contract "MyTest"

# With verbose output
./target/release/cbse --root . --function "test_" -vv

# With prover mode (outputs signed attestation)
./target/release/cbse --root . --prover-mode --private-key $PROVER_KEY
```

#### CLI Options

```
cbse [OPTIONS]

OPTIONS:
    --root <PATH>              Project root directory [default: .]
    --match-contract <REGEX>   Filter contracts by regex
    --match-test <REGEX>       Filter tests by regex
    --function <PREFIX>        Function prefix [default: (test|check|invariant)_]
    
    --loop-bound <N>           Loop unrolling bound [default: 2]
    --depth <N>                Max path length [default: unlimited]
    --width <N>                Max number of paths [default: unlimited]
    
    --solver <NAME>            SMT solver: z3, yices, cvc5 [default: yices]
    --solver-timeout <MS>      Solver timeout in milliseconds
    
    --prover-mode              Output signed attestation JSON
    --private-key <HEX>        Private key for signing
    
    -v, --verbose              Increase verbosity (-v, -vv, -vvv)
    --debug                    Enable debug output
```

#### API Endpoints

**POST `/verify`**

Request:
```json
{
  "contract_source": "// Solidity contract code...",
  "spec_source": "// Test specification code...",
  "contract_name": "MyContract"
}
```

Response:
```json
{
  "job_id": "uuid",
  "status": "Success",
  "message": "Verification completed successfully.",
  "attestation": {
    "result_hash": "0x...",
    "passed": true,
    "bytecode_hash": "0x...",
    "spec_hash": "0x...",
    "timestamp": 1234567890,
    "signature": "0x..."
  }
}
```

#### Architecture

```
FM-rust-cloud/
├── crates/
│   ├── cbse/              # Main CLI binary
│   ├── cbse-coordinator/  # HTTP API server
│   ├── cbse-sevm/         # Symbolic EVM engine
│   ├── cbse-bitvec/       # Symbolic bitvectors
│   ├── cbse-solver/       # Z3 integration
│   ├── cbse-protocol/     # Attestation signing
│   ├── cbse-config/       # Configuration
│   └── ...                # 20+ supporting crates
└── Cargo.toml             # Workspace manifest
```

---

### Smart Contracts (`smart-contracts/`)

The on-chain attestation registry deployed on an EVM chain.

#### Contracts

- **`AttestationRegistry.sol`** - Stores verification attestations on-chain

#### Installation

```bash
cd smart-contracts

# Install Foundry (if not installed)
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Install dependencies
forge install
```

#### Build

```bash
forge build
```

#### Test

```bash
forge test
```

#### Deployment

**Local (Anvil):**

```bash
# Terminal 1: Start Anvil
anvil

# Terminal 2: Deploy
forge script script/DeployRegistry.s.sol:DeployRegistry \
  --rpc-url http://127.0.0.1:8545 \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
  --broadcast
```

**EVM Chain:**

```bash
# Set environment variables
export PRIVATE_KEY=your_deployer_private_key
export RPC_URL=https://eth-sepolia.public.blastapi.io

# Deploy
forge script script/DeployRegistry.s.sol:DeployRegistry \
  --rpc-url $RPC_URL \
  --private-key $PRIVATE_KEY \
  --broadcast \
  --verify
```

#### Contract Interface

```solidity
interface IAttestationRegistry {
    // Events
    event VerificationAttested(
        bytes32 indexed resultHash,
        address indexed prover,
        bool passed,
        bytes32 contractHash
    );

    // Commit a verification attestation
    function commitAttestation(
        bytes32 resultHash,
        bool passed,
        bytes32 contractHash,
        uint8 v, bytes32 r, bytes32 s
    ) external;

    // Prover management (owner only)
    function setProver(address prover, bool status) external;
    function isProver(address) external view returns (bool);
}
```

#### Authorizing Provers

After deployment, authorize your CBSE coordinator's prover address:

```bash
cast send $REGISTRY_ADDRESS "setProver(address,bool)" $PROVER_ADDRESS true \
  --rpc-url $RPC_URL \
  --private-key $OWNER_PRIVATE_KEY
```

---

## Full Stack Deployment

### Production Architecture

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│     Vercel       │     │   Cloud Server   │     │   BNB Testnet    │
│   (Frontend)     │────▶│  (Coordinator)   │────▶│   (Registry)     │
│   Port 443       │     │   Port 3001      │     │   Contract       │
└──────────────────┘     └──────────────────┘     └──────────────────┘
```

### Step-by-Step Production Deployment

#### 1. Deploy Smart Contracts

```bash
cd smart-contracts
forge script script/DeployRegistry.s.sol:DeployRegistry \
  --rpc-url https://rpc.mantle.xyz \ EVM Chain 
  --private-key $DEPLOYER_KEY \
  --broadcast --verify

# Note the deployed contract address
```

#### 2. Deploy Coordinator

On your cloud server (e.g., AWS EC2, DigitalOcean):

```bash
# Install dependencies
sudo apt update
sudo apt install -y build-essential z3 libz3-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Clone and build
git clone https://github.com/your-org/veripro.git
cd veripro/FM-rust-cloud
cargo build --release

# Create systemd service
sudo tee /etc/systemd/system/cbse-coordinator.service << EOF
[Unit]
Description=CBSE Coordinator
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/veripro/FM-rust-cloud
Environment=PROVER_PRIVATE_KEY=your_prover_key
Environment=FORGE_STD_PATH=/home/ubuntu/veripro/smart-contracts/lib/forge-std
ExecStart=/home/ubuntu/veripro/FM-rust-cloud/target/release/cbse-coordinator
Restart=always

[Install]
WantedBy=multi-user.target
EOF

# Start service
sudo systemctl enable cbse-coordinator
sudo systemctl start cbse-coordinator
```

#### 3. Deploy Frontend

```bash
cd cbse-frontend

# Set environment variables in Vercel dashboard:
# - GOOGLE_API_KEY
# - NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID

# Update next.config.ts with coordinator URL
# destination: 'https://your-coordinator-server.com/verify'

# Deploy
vercel --prod
```

#### 4. Authorize Prover

```bash
# Get the prover address from your PROVER_PRIVATE_KEY
PROVER_ADDRESS=$(cast wallet address --private-key $PROVER_PRIVATE_KEY)

# Authorize on the registry
cast send $REGISTRY_ADDRESS "setProver(address,bool)" $PROVER_ADDRESS true \
  --rpc-url https://rpc.mantle.xyz \
  --private-key $OWNER_KEY
```

---

## Usage Guide

### Writing Specifications

VeriPro uses Foundry-style test contracts as specifications:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "./MyToken.sol";

contract MyTokenTest is Test {
    MyToken token;

    function setUp() public {
        token = new MyToken(1000000);
    }

    // Invariant: balance never exceeds supply
    function test_balanceNeverExceedsSupply(address user) public view {
        assertLe(token.balanceOf(user), token.totalSupply());
    }

    // Functional: transfer updates balances correctly
    function test_transferUpdatesBalances(
        address from,
        address to,
        uint256 amount
    ) public {
        vm.assume(from != to);
        vm.assume(token.balanceOf(from) >= amount);

        uint256 fromBefore = token.balanceOf(from);
        uint256 toBefore = token.balanceOf(to);

        vm.prank(from);
        token.transfer(to, amount);

        assertEq(token.balanceOf(from), fromBefore - amount);
        assertEq(token.balanceOf(to), toBefore + amount);
    }
}
```

### Verification Flow

1. **Create Project** - Upload or paste your contract code
2. **Write Specification** - Define properties to verify (or use AI generation)
3. **Run Verification** - CBSE explores all execution paths
4. **Review Results** - See passed/failed properties with counterexamples
5. **Commit On-Chain** - Publish attestation to Sepolia Testnet (optional)

### VM Cheatcodes

```solidity
vm.assume(bool)           // Add path constraint
vm.prank(address)         // Set msg.sender for next call
vm.deal(address, uint)    // Set ETH balance
vm.roll(uint)             // Set block.number
vm.warp(uint)             // Set block.timestamp
vm.expectRevert()         // Expect next call to revert
```

---blockchain

## Troubleshooting

### Common Issues

**Coordinator won't start:**
```bash
# Check Z3 is installed
z3 --version

# Check Foundry is installed
forge --version
```

**Verification fails with "forge-std not found":**
```bash
# Set the forge-std path
export FORGE_STD_PATH=/path/to/forge-std

# Or ensure it's in the project's lib/ directory
```

**Frontend can't connect to coordinator:**
```bash
# Check coordinator is running
curl http://127.0.0.1:3001/health

# Check next.config.ts proxy settings
```

**Attestation commitment fails:**
```bash
# Verify prover is authorized
cast call $REGISTRY_ADDRESS "isProver(address)" $PROVER_ADDRESS

# Check signature format
```

---

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests (`cargo test`, `npm run lint`, `forge test`)
5. Submit a pull request

---

## License

VeriPro is available under a **dual license** model:

### Option 1: MIT License (Open Source)
Free for open-source projects, personal use, and educational purposes.
- **Requirement**: Include attribution to VeriPro in your documentation

### Option 2: Commercial License
For commercial use without attribution requirements.
- **Includes**: Priority support, SLA, custom integrations, indemnification
- **Contact**: [licensing@your-domain.com]

| Use Case               | License                |
| ---------------------- | ---------------------- |
| Personal/Educational   | MIT (with attribution) |
| Open-source projects   | MIT (with attribution) |
| Commercial SaaS        | Commercial             |
| Enterprise/White-label | Commercial             |

See [LICENSE](./LICENSE) for full details.

---

## Links

- [Documentation](https://your-domain.com/docs)
- [Demo](https://your-domain.com/demo)
- [GitHub Issues](https://github.com/your-org/veripro/issues)
- [Commercial Licensing](https://your-domain.com/licensing)

---

Built with Rust, Next.js, and Solidity.
