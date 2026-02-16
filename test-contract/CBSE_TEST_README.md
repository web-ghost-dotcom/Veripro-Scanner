# CBSE Test Example - SimpleVault

This directory contains a simple smart contract and symbolic tests to demonstrate CBSE (Complete Blockchain Symbolic Executor).

## Contract: SimpleVault

Location: `src/SimpleVault.sol`

A minimal vault contract that allows users to:
- Deposit funds
- Withdraw funds  
- Check balances

### Key Features:
- Balance tracking per address
- Total deposits tracking
- Basic input validation

## Symbolic Tests

Location: `test/SimpleVault.t.sol`

The test suite includes 6 symbolic test functions:

1. **`testDeposit(uint256 amount)`** - Verifies deposit increases balance correctly
2. **`testWithdraw(uint256 depositAmount, uint256 withdrawAmount)`** - Verifies withdrawal decreases balance correctly
3. **`testWithdrawOverflow(uint256 depositAmount, uint256 withdrawAmount)`** - Tests that withdrawing more than balance reverts
4. **`testBalanceInvariant(uint256 amount)`** - Checks that user balance never exceeds total deposits
5. **`testZeroDeposit()`** - Verifies zero deposits are rejected
6. **`testMultipleDeposits(uint256 amount1, uint256 amount2)`** - Tests multiple deposits from same user

### Symbolic Testing Features Demonstrated:

- **Symbolic inputs**: Function parameters are symbolic values (not concrete)
- **Assumptions**: `vm.assume()` constrains the input space
- **Assertions**: `assert()` statements that should hold for ALL valid inputs
- **Cheatcodes**: `vm.prank()`, `vm.expectRevert()` for test setup

## Building

```bash
cd /Users/mac/Downloads/halmos/test-contract
forge build
```

This generates artifacts in `out/` directory including:
- `out/SimpleVault.sol/SimpleVault.json`
- `out/SimpleVault.t.sol/SimpleVaultTest.json`

## Running CBSE

```bash
# Run all tests in SimpleVaultTest
cbse --contract SimpleVaultTest -vvv

# Run specific function
cbse --contract SimpleVaultTest --function testDeposit -vvv

# Run with more parallel processes
cbse --contract SimpleVaultTest -j 4 -vv
```

## Current CBSE Status

✅ **Working:**
- CLI argument parsing (40+ options)
- Build artifact discovery
- Contract loading from Forge artifacts
- Z3 solver integration
- Basic framework and architecture

🚧 **In Progress:**
- ABI parsing to detect test functions (currently returns empty list)
- Full EVM opcode implementations (only 4 basic opcodes: ADD, PUSH1, SLOAD, SSTORE)
- Symbolic path exploration
- Cheatcode implementations (vm.assume, vm.prank, etc.)
- Counterexample generation

## Expected Behavior

When fully implemented, CBSE should:

1. **Discover test functions** by parsing the ABI from `SimpleVaultTest.json`
2. **Generate symbolic calldata** for each test function's parameters
3. **Execute symbolically** exploring all possible paths
4. **Collect path constraints** using Z3
5. **Verify assertions** on all paths
6. **Report results**:
   - ✅ PASS if assertions hold on all paths
   - ❌ FAIL with counterexample if any path violates an assertion

### Example Expected Output:

```
╔══════════════════════════════════════════════════╗
║              CBSE - Symbolic Testing             ║
║    Complete Blockchain Symbolic Executor         ║
╚══════════════════════════════════════════════════╝

INFO Running CBSE on project: "."
INFO Found 1 test contract(s)
INFO Testing contract: SimpleVaultTest
INFO Found 6 test function(s)

Running: testDeposit
  ✅ PASS (explored 2 paths, 0.5s)

Running: testWithdraw
  ✅ PASS (explored 4 paths, 1.2s)

Running: testWithdrawOverflow
  ✅ PASS (explored 2 paths, 0.8s)

Running: testBalanceInvariant
  ✅ PASS (explored 1 path, 0.3s)

Running: testZeroDeposit
  ✅ PASS (explored 1 path, 0.2s)

Running: testMultipleDeposits
  ✅ PASS (explored 3 paths, 1.5s)

═══════════════════════════════════════
           Test Results
═══════════════════════════════════════

Total tests:  6
Passed:       6
Failed:       0
Duration:     4.5s

✅ All tests passed!
```

## Next Steps for CBSE Development

To make CBSE fully functional:

1. **Parse ABI in `find_test_functions()`** - Extract function signatures from JSON
2. **Implement EVM opcodes** - Complete the 136 missing opcodes
3. **Add symbolic execution engine** - Path exploration with constraint tracking
4. **Implement Foundry cheatcodes** - vm.assume, vm.prank, vm.expectRevert, etc.
5. **Add counterexample generation** - When assertions fail, provide concrete inputs
6. **Optimize solver queries** - Cache results, minimize constraints

## Testing the Framework

Even though symbolic execution isn't complete, you can verify CBSE is working:

```bash
# Verify installation
cbse --version  # Should show: cbse 0.0.1

# Check it finds the contract
cbse --contract SimpleVaultTest -vvv

# Try different options
cbse --help
cbse --list-contracts
cbse --depth 10 --loop-bound 3
```

## Documentation

See also:
- `/Users/mac/Downloads/halmos/FM-rust/README.md` - Project overview
- `/Users/mac/Downloads/halmos/FM-rust/ARCHITECTURE.md` - Technical architecture
- `/Users/mac/Downloads/halmos/FM-rust/TESTING_GUIDE.md` - Complete CLI usage guide
