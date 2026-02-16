#!/bin/bash
# Test the full Protocol Flow: User -> Coordinator -> Prover -> Attestation

echo "Starting CBSE Protocol Job Simulation..."

# Sample Solidity Contract
CONTRACT_SOURCE='// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
contract Vault {
    uint256 public balance;
    function deposit() external payable {
        balance += msg.value;
    }
}'

# Sample Spec
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

# JSON Payload
JSON_PAYLOAD=$(jq -n \
                  --arg c "$CONTRACT_SOURCE" \
                  --arg s "$SPEC_SOURCE" \
                  --arg n "Vault" \
                  '{contract_source: $c, spec_source: $s, contract_name: $n}')

echo "Submitting Job to Coordinator at http://localhost:3001/verify..."

# Send Request
RESPONSE=$(curl -v -X POST http://localhost:3001/verify \
     -H "Content-Type: application/json" \
     -d "$JSON_PAYLOAD")

echo ""
echo "Response Received:"
# Print raw response for debugging
echo "$RESPONSE"

# Check if response is valid JSON
if ! echo "$RESPONSE" | jq empty > /dev/null 2>&1; then
    echo "❌ Error: Response is not valid JSON."
    echo "Raw Response: $RESPONSE"
    exit 1
fi

echo "$RESPONSE" | jq .

# Check for attestation
ATTESTATION=$(echo "$RESPONSE" | jq -r .attestation)

if [ "$ATTESTATION" != "null" ] && [ -n "$ATTESTATION" ]; then
    echo ""
    echo "✅ VERIFICATION SUCCESSFUL"
    echo "This is a cryptographically signed proof."
    echo "Next Step: Submit this hash to Blockchain."
else
    echo ""
    echo "❌ VERIFICATION PENDING/FAILED"
    echo "Check coordinator logs."
fi
