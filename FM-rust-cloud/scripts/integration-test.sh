#!/bin/bash
set -e

# ==============================================================================
# INTEGRATION TEST SUITE: CBSE Decentralized Verification Protocol
# ==============================================================================
# This script spins up a local Devnet (Anvil), deploys the Protocol Registry,
# starts the Coordinator Node, runs a Prover Job, and settles the proof on-chain.
# ==============================================================================

# Cleanup function
cleanup() {
    echo "🧹 Cleaning up..."
    if [ -n "$ANVIL_PID" ]; then kill $ANVIL_PID; fi
    if [ -n "$COORD_PID" ]; then kill $COORD_PID; fi
}
trap cleanup EXIT

echo "🚀 Starting Integration Test Suite..."

# 1. Start Local Devnet (Anvil)
if ! command -v anvil &> /dev/null; then
    echo "❌ 'anvil' not found. Please install Foundry."
    exit 1
fi

echo "🔗 Starting Anvil..."
anvil --port 8545 > anvil_log.txt 2>&1 &
ANVIL_PID=$!
sleep 3 # Wait for Anvil to start

# 2. Deploy AttestationRegistry
echo "� Building Prover Binary..."
cd ../FM-rust-cloud
export LIBRARY_PATH=/usr/local/lib:$LIBRARY_PATH
export DYLD_LIBRARY_PATH=/usr/local/lib:$DYLD_LIBRARY_PATH
export Z3_SYS_Z3_HEADER=/usr/local/include/z3.h
cargo build --bin cbse
if [ $? -ne 0 ]; then
    echo "❌ Failed to build cbse binary."
    exit 1
fi

echo "�📜 Deploying AttestationRegistry to Anvil..."
cd ../smart-contracts
export PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 # Anvil Account #0
RPC="http://127.0.0.1:8545"

# Run deployment script and capture output
DEPLOY_OUT=$(forge script script/DeployRegistry.s.sol --rpc-url $RPC --broadcast --private-key $PRIVATE_KEY)
echo "$DEPLOY_OUT"

# Extract contract address (Naive grep, robust enough for MVP)
REGISTRY_ADDR=$(echo "$DEPLOY_OUT" | grep "AttestationRegistry deployed at:" | awk '{print $4}')

if [ -z "$REGISTRY_ADDR" ]; then
    echo "❌ Failed to capture Registry Address."
    exit 1
fi
echo "✅ Registry Deployed at: $REGISTRY_ADDR"

cd ../FM-rust-cloud

# 3. Start Coordinator
echo "🧠 Starting Coordinator Node..."
export CBSE_BINARY="$(pwd)/target/debug/cbse"
export PROVER_PRIVATE_KEY=$PRIVATE_KEY
export SMART_CONTRACTS_LIB_PATH=$(cd ../smart-contracts/lib && pwd)

cargo run --bin cbse-coordinator > coordinator_log.txt 2>&1 &
COORD_PID=$!
sleep 15 # Wait for compilation and startup

# 4. Run Job Simulation (User Role)
echo "👤 User submitting Verification Job..."

# Define Inputs inline to ensure clean output capture
CONTRACT_SOURCE='// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
contract Vault {
    uint256 public balance;
    function deposit() external payable {
        balance += msg.value;
    }
}'

SPEC_SOURCE='// SPDX-License-Identifier: MIT
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
}'

JSON_PAYLOAD=$(jq -n \
                  --arg c "$CONTRACT_SOURCE" \
                  --arg s "$SPEC_SOURCE" \
                  --arg n "Vault" \
                  '{contract_source: $c, spec_source: $s, contract_name: $n}')

# Send Request
OUTPUT=$(curl -s -X POST http://localhost:3001/verify \
     -H "Content-Type: application/json" \
     -d "$JSON_PAYLOAD")

echo "$OUTPUT" | jq .

# 5. Extract Attestation Data for On-Chain Settlement
echo "🔍 Parsing Attestation..."
ATTESTATION_JSON=$(echo "$OUTPUT" | jq -r '.attestation')

if [ "$ATTESTATION_JSON" == "null" ]; then
    echo "❌ Failed to get attestation."
    cat coordinator_log.txt
    exit 1
fi

RESULT_HASH=$(echo "$ATTESTATION_JSON" | jq -r '.result_hash')
PROVER_ADDR=$(echo "$ATTESTATION_JSON" | jq -r '.prover_address')
PASSED=$(echo "$ATTESTATION_JSON" | jq -r '.payload.passed')
CONTRACT_HASH=$(echo "$ATTESTATION_JSON" | jq -r '.payload.contract_bytecode_hash')
SIGNATURE=$(echo "$ATTESTATION_JSON" | jq -r '.signature') # This is raw bytes hex

echo "   Result Hash: $RESULT_HASH"
echo "   Prover: $PROVER_ADDR"
echo "   Passed: $PASSED"
echo "   Sig: ${SIGNATURE:0:20}..."

# 6. Settle on Chain (Prover/Relayer Role)
echo "⛓️  Settling Proof on Blockchain (Anvil)..."

# Helper to split signature (r, s, v) from the raw bytes
# Rust signature is likely 64 bytes (r,s) or 65 (r,s,v). We need to check formatting.
# For this MVP, let's assume standard RSV split if possible, or just fail safely if we need more python glue.
# We will use 'cast' to call commitAttestation
# function commitAttestation(bytes32 resultHash, bool passed, bytes32 contractHash, uint8 v, bytes32 r, bytes32 s)

# Verify Signature Length
LEN=${#SIGNATURE}
if [ "$LEN" -eq 128 ]; then
    # 64 bytes (128 hex chars) -> r, s. No V?
    echo "⚠️ Signature format might be missing V (recovery id). Assuming v=27/28"
    R="0x${SIGNATURE:0:64}"
    S="0x${SIGNATURE:64:64}"
    V=27 # valid guess for now
elif [ "$LEN" -eq 130 ]; then
    # 65 bytes
    R="0x${SIGNATURE:0:64}"
    S="0x${SIGNATURE:64:64}"
    V_HEX="${SIGNATURE:128:2}"
    V=$((16#$V_HEX)) # Convert hex to dec
else 
    R="0x${SIGNATURE:0:64}"
    S="0x${SIGNATURE:64:64}"
    V=27
    echo "⚠️ Unknown sig length $LEN. Trying anyway."
fi

# Call the contract
cast send $REGISTRY_ADDR "commitAttestation(bytes32,bool,bytes32,uint8,bytes32,bytes32)" \
    "$RESULT_HASH" "$PASSED" "$CONTRACT_HASH" "$V" "$R" "$S" \
    --rpc-url $RPC --private-key $PRIVATE_KEY

# 7. Verify Event Emission
echo "✅ Proof Settled. Checking for events..."
EVENTS=$(cast logs --address $REGISTRY_ADDR --from-block 0 --rpc-url $RPC)

if [[ $EVENTS == *"VerificationAttested"* ]] || [[ $EVENTS == *"0x3baabc024691255a98d9eaae06a19752f00ded888bdc9e38002e1c49990119ca"* ]]; then
    echo "🎉 SUCCESS: VerificationAttested event found on chain!"
else
    echo "❌ FAILURE: Event not found."
    echo "$EVENTS"
    exit 1
fi

echo "==========================================="
echo "✅ INTEGRATION TEST COMPLETED SUCCESSFULLY"
echo "==========================================="
