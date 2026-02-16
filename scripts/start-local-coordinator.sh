#!/bin/bash
set -e

# Change to the script's directory
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$DIR/.."

echo "🚀 Setting up Local Coordinator for BNB Testnet..."

# 1. Load Deployer Private Key
if [ -f "$PROJECT_ROOT/smart-contracts/.env" ]; then
    # Load env vars
    source "$PROJECT_ROOT/smart-contracts/.env"
    export PROVER_PRIVATE_KEY=$PRIVATE_KEY
    if [ -z "$PROVER_PRIVATE_KEY" ]; then
        echo "❌ Error: PRIVATE_KEY not found in .env!"
        exit 1
    fi
    echo "✅ Loaded PRIVATE_KEY..."
else
    echo "❌ Error: smart-contracts/.env not found!"
    exit 1
fi

# 2. Build CBSE Engine (if not built)
CBSE_BIN="$PROJECT_ROOT/FM-rust-cloud/target/debug/cbse"
if [ ! -f "$CBSE_BIN" ]; then
    echo "🛠 Building CBSE Engine..."
    cd "$PROJECT_ROOT/FM-rust-cloud"
    cargo build --bin cbse
fi

export CBSE_BINARY="$CBSE_BIN"
export PROVER_MODE=true

# 3. Start Coordinator
echo "🌐 Starting Coordinator on port 3001..."

cd "$PROJECT_ROOT/FM-rust-cloud"
cargo run --bin cbse-coordinator
