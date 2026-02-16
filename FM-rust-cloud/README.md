# CBSE - Complete Blockchain Symbolic Executor

A fast, memory-safe symbolic execution engine for Ethereum smart contracts, written in Rust. CBSE performs symbolic testing of Solidity contracts to find bugs, verify assertions, and check invariants.

## Overview

CBSE explores all possible execution paths through your smart contract, using Z3 SMT solver to verify that your assertions hold for all possible inputs—not just the ones you test.

```
Solidity: function test(uint x) { assert(x < 100); }
          │
          ▼
Bytecode: CALLDATALOAD PUSH 100 LT ...
          │
          ▼
Symbolic: x = Z3.BitVec('x', 256)
          constraint: x < 100
          │
          ▼
Z3 Solve: Is there an x where !(x < 100)?
          │
          ▼
Result:   Pass (assertion holds for all valid x)
```

## Features

- Complete EVM opcode support
- Z3 Array-based symbolic storage for mappings
- Path exploration with configurable bounds
- Foundry test integration
- Signed verification attestations
- HTTP coordinator for API access

## Prerequisites

```bash
# Rust 1.70+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Z3 SMT Solver
# macOS:
brew install z3

# Ubuntu/Debian:
sudo apt-get install z3 libz3-dev

# Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

## Installation

```bash
# Build the project
cargo build --release

# Binaries are at:
# - target/release/cbse           (CLI)
# - target/release/cbse-coordinator (HTTP API)
```

## Quick Start

### CLI Usage

```bash
# Run symbolic tests on a Foundry project
./target/release/cbse --root /path/to/project --match-contract "MyTest"

# With verbose output
./target/release/cbse --root . --function "test_" -vv

# With prover mode (outputs signed attestation)
./target/release/cbse --root . --prover-mode --private-key $PROVER_KEY
```

### Coordinator (HTTP API)

```bash
# Start the coordinator
cargo run --bin cbse-coordinator

# Or in production
./target/release/cbse-coordinator
```

The coordinator listens on `http://127.0.0.1:3001`.

## CLI Options

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

## Environment Variables

```bash
# Path to forge-std library (auto-detected if not set)
export FORGE_STD_PATH=/path/to/forge-std

# Prover private key for signing attestations
export PROVER_PRIVATE_KEY=your_private_key_hex

# Custom CBSE binary path (for coordinator)
export CBSE_BINARY=/path/to/cbse
```

## Coordinator API

### POST `/verify`

Verify a contract with a specification.

**Request:**

```json
{
  "contract_source": "// SPDX-License-Identifier: MIT\npragma solidity ^0.8.13;\n\ncontract Counter {\n    uint256 public number;\n    function increment() public { number++; }\n}",
  "spec_source": "// SPDX-License-Identifier: MIT\npragma solidity ^0.8.13;\n\nimport \"forge-std/Test.sol\";\nimport \"./Counter.sol\";\n\ncontract CounterTest is Test {\n    Counter counter;\n    function setUp() public { counter = new Counter(); }\n    function test_increment() public {\n        counter.increment();\n        assertEq(counter.number(), 1);\n    }\n}",
  "contract_name": "Counter"
}
```

**Response (Success):**

```json
{
  "job_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "status": "Success",
  "message": "Verification completed successfully.\n\nRunning 1 test for CounterTest\n  Pass: test_increment\n\nSymbolic test result: 1 passed; 0 failed",
  "attestation": {
    "result_hash": "0x...",
    "passed": true,
    "bytecode_hash": "0x...",
    "spec_hash": "0x...",
    "timestamp": 1704067200,
    "signature": "0x..."
  }
}
```

**Response (Failure):**

```json
{
  "job_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "status": "Failed",
  "message": "Verification failed.\n\nRunning 1 test for CounterTest\n  Fail: test_overflow\n    Counterexample: x = 115792089237316195423570985008687907853269984665640564039457584007913129639935",
  "attestation": null
}
```

### GET `/health`

Health check endpoint.

**Response:**

```json
{
  "status": "ok"
}
```

## Writing Specifications

CBSE uses Foundry-style test contracts as specifications:

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

    // Symbolic test - verified for ALL possible values
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

### Supported Cheatcodes

```solidity
vm.assume(bool)           // Add path constraint
vm.prank(address)         // Set msg.sender for next call
vm.deal(address, uint)    // Set ETH balance
vm.roll(uint)             // Set block.number
vm.warp(uint)             // Set block.timestamp
vm.expectRevert()         // Expect next call to revert
```

## Architecture

```
FM-rust-cloud/
├── crates/
│   ├── cbse/              # Main CLI binary
│   ├── cbse-coordinator/  # HTTP API server
│   ├── cbse-sevm/         # Symbolic EVM engine
│   ├── cbse-bitvec/       # Symbolic bitvectors
│   ├── cbse-bytevec/      # Byte vector operations
│   ├── cbse-solver/       # Z3 integration
│   ├── cbse-protocol/     # Attestation signing
│   ├── cbse-config/       # Configuration
│   ├── cbse-cheatcodes/   # Foundry cheatcode support
│   ├── cbse-contract/     # Contract loading
│   ├── cbse-build/        # Forge build integration
│   └── ...                # Additional supporting crates
├── src/                   # Shared library code
├── scripts/               # Utility scripts
└── Cargo.toml             # Workspace manifest
```

## Deployment

### Systemd Service

Create `/etc/systemd/system/cbse-coordinator.service`:

```ini
[Unit]
Description=CBSE Coordinator
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/opt/cbse
Environment=PROVER_PRIVATE_KEY=your_prover_key
Environment=FORGE_STD_PATH=/opt/forge-std
ExecStart=/opt/cbse/cbse-coordinator
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Start the service:

```bash
sudo systemctl enable cbse-coordinator
sudo systemctl start cbse-coordinator
sudo systemctl status cbse-coordinator
```

### Docker

```dockerfile
FROM rust:1.75 AS builder
RUN apt-get update && apt-get install -y z3 libz3-dev
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y z3 libz3-4 ca-certificates
COPY --from=builder /app/target/release/cbse-coordinator /usr/local/bin/
EXPOSE 3001
CMD ["cbse-coordinator"]
```

Build and run:

```bash
docker build -t cbse-coordinator .
docker run -p 3001:3001 \
  -e PROVER_PRIVATE_KEY=$PROVER_KEY \
  -e FORGE_STD_PATH=/forge-std \
  -v /path/to/forge-std:/forge-std \
  cbse-coordinator
```

## Testing

```bash
# Run all Rust tests
cargo test

# Run specific crate tests
cargo test -p cbse-sevm

# Run integration tests
cargo test --test test_new_opcodes

# Test coordinator with curl
curl -X POST http://127.0.0.1:3001/verify \
  -H "Content-Type: application/json" \
  -d '{"contract_source": "...", "spec_source": "...", "contract_name": "Counter"}'
```

## Performance

- Speed: ~9.6 tests/second
- Memory: Efficient with Rust's ownership system
- Scalability: Configurable bounds for depth/width

## Troubleshooting

### Z3 not found

```bash
# Verify Z3 installation
z3 --version

# Check library path
ldconfig -p | grep z3
```

### Forge build fails

```bash
# Verify Foundry installation
forge --version

# Check forge-std path
ls $FORGE_STD_PATH
```

### Coordinator won't start

```bash
# Check port availability
lsof -i :3001

# Check logs
journalctl -u cbse-coordinator -f
```

## License

AGPL-3.0

## Credits

- Z3 Theorem Prover by Microsoft Research
- Foundry by Paradigm
- Inspired by Halmos
