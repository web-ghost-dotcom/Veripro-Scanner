# CBSE Architecture Guide: Complete Blockchain Symbolic Execution

## Table of Contents
1. [High-Level Overview](#high-level-overview)
2. [Execution Modes](#execution-modes)
3. [System Architecture](#system-architecture)
4. [Crate Responsibilities](#crate-responsibilities)
5. [Execution Flow](#execution-flow)
6. [Data Flow Diagrams](#data-flow-diagrams)
7. [User Guide](#user-guide)

---

## High-Level Overview

**CBSE (Complete Blockchain Symbolic Executor)** is a Rust-based symbolic execution engine for Ethereum smart contracts. It analyzes Solidity code by exploring all possible execution paths to find vulnerabilities, assertion failures, and edge cases that traditional testing might miss.

### Key Capabilities
- **Symbolic Execution**: Explores all code paths using symbolic values instead of concrete inputs
- **Automatic Bug Detection**: Finds overflows, underflows, assertion failures, and reverts
- **Path Exploration**: Discovers counterexamples that violate invariants
- **Dual Execution Modes**: Run locally or offload to remote cloud servers via SSH

### Core Philosophy
```
Traditional Testing:  test(5) ✓  test(10) ✓  test(100) ✓  ... (limited coverage)
Symbolic Execution:   test(X) where X ∈ [0, 2^256-1]  (complete coverage)
```

---

## Execution Modes

### 1. Local Mode (Default)
Execute everything on your local machine:
```bash
cbse --function "test"
```

**Use Cases**:
- Development and debugging
- Small to medium contracts
- Quick iteration cycles
- No network latency concerns

### 2. SSH Cloud Mode
Compile locally, execute remotely:
```bash
cbse --ssh --ssh-host node10@node10 --function "test"
```

**Use Cases**:
- Large contract suites requiring heavy computation
- Offloading Z3 solver work to powerful servers
- Parallel execution across multiple nodes (future)
- Resource-constrained local machines

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         CBSE Architecture Overview                       │
└─────────────────────────────────────────────────────────────────────────┘

                    ┌──────────────────┐
                    │  Solidity Code   │
                    │   (.sol files)   │
                    └────────┬─────────┘
                             │
                             ▼
         ┌───────────────────────────────────────┐
         │     PHASE 1: COMPILATION              │
         │  (Always happens locally)             │
         └───────────────────────────────────────┘
                             │
                 ┌───────────┴───────────┐
                 │   cbse-build          │
                 │   Forge Integration   │
                 └───────────┬───────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │  Build Artifact  │
                    │  - Bytecode      │
                    │  - ABI           │
                    │  - Storage       │
                    │  - Metadata      │
                    └────────┬─────────┘
                             │
                ┌────────────┴────────────┐
                │                         │
                ▼                         ▼
    ┌──────────────────┐      ┌──────────────────────┐
    │   LOCAL MODE     │      │   SSH CLOUD MODE     │
    └──────────────────┘      └──────────────────────┘
                │                         │
                │                         ▼
                │              ┌────────────────────┐
                │              │  cbse-remote       │
                │              │  SSH Upload        │
                │              └─────────┬──────────┘
                │                        │
                │                        ▼
                │              ┌────────────────────┐
                │              │  Remote Server     │
                │              │  (Worker Mode)     │
                │              └─────────┬──────────┘
                │                        │
                └────────────┬───────────┘
                             │
                             ▼
         ┌───────────────────────────────────────┐
         │     PHASE 2: SYMBOLIC EXECUTION       │
         │  (Local or Remote)                    │
         └───────────────────────────────────────┘
                             │
                 ┌───────────┴───────────┐
                 │   cbse-sevm           │
                 │   Symbolic EVM        │
                 └───────────┬───────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │  Path Exploration│
                    │  - Fork on branch│
                    │  - Track paths   │
                    │  - Collect traces│
                    └────────┬─────────┘
                             │
                             ▼
                 ┌───────────────────────┐
                 │   cbse-solver (Z3)    │
                 │   Constraint Solving  │
                 └───────────┬───────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │  Test Results    │
                    │  - Pass/Fail     │
                    │  - Traces        │
                    │  - Counterexample│
                    └────────┬─────────┘
                             │
                ┌────────────┴────────────┐
                │                         │
                ▼                         ▼
    ┌──────────────────┐      ┌──────────────────────┐
    │  Display Locally │      │  Download Results    │
    │  cbse-ui         │      │  cbse-remote         │
    └──────────────────┘      └──────────┬───────────┘
                                          │
                                          ▼
                              ┌──────────────────────┐
                              │  Display Locally     │
                              │  cbse-ui             │
                              └──────────────────────┘
```

---

## Crate Responsibilities

### Core Execution Crates

#### **cbse** (Main Binary)
- **Purpose**: Entry point and orchestration
- **Responsibilities**:
  - CLI argument parsing
  - Mode selection (local vs SSH)
  - Test discovery and filtering
  - Result aggregation and display
- **Key Files**: `src/main.rs`
- **Dependencies**: All other crates

#### **cbse-sevm** (Symbolic EVM)
- **Purpose**: Core symbolic execution engine
- **Responsibilities**:
  - EVM opcode interpretation (ADD, MUL, SLOAD, etc.)
  - Symbolic state management (stack, memory, storage)
  - Path forking on conditional branches (JUMPI)
  - Call/Return handling
  - Gas tracking
- **Key Structures**:
  - `SEVM<'ctx>`: Main executor
  - `State<'ctx>`: EVM state snapshot
  - `Path<'ctx>`: Execution path with constraints
- **Why Needed**: Executes bytecode symbolically to explore all paths

#### **cbse-solver** (Z3 Integration)
- **Purpose**: Constraint solving and SAT checking
- **Responsibilities**:
  - Z3 solver configuration
  - Query optimization
  - Model generation (counterexamples)
  - Timeout handling
- **Key Functions**:
  - `check_sat()`: Determine if path is feasible
  - `get_model()`: Extract concrete values
- **Why Needed**: Determines which execution paths are possible

#### **cbse-config** (Configuration)
- **Purpose**: Central configuration management
- **Responsibilities**:
  - CLI flags parsing
  - Default values
  - SSH settings
  - Solver options (timeout, memory)
  - Verbosity levels
- **Key Structure**: `Config` with 50+ fields
- **Why Needed**: Single source of truth for all settings

### Data Structure Crates

#### **cbse-bitvec** (Bit Vectors)
- **Purpose**: Symbolic bit vector operations
- **Responsibilities**:
  - Wrapping Z3 bit vectors
  - Arithmetic operations (add, mul, div)
  - Bitwise operations (and, or, xor, shl, shr)
  - Comparison operations
  - Concrete value extraction
- **Key Type**: `CbseBitVec<'ctx>`
- **Why Needed**: EVM operates on 256-bit words; this provides symbolic arithmetic

#### **cbse-bytevec** (Byte Vectors)
- **Purpose**: Symbolic byte sequences
- **Responsibilities**:
  - Memory and storage representation
  - Dynamic-length byte arrays
  - Concrete/symbolic byte mixing
  - Slicing and concatenation
- **Key Type**: `ByteVec<'ctx>`
- **Why Needed**: Models EVM memory and calldata symbolically

#### **cbse-contract** (Contract State)
- **Purpose**: Contract execution context
- **Responsibilities**:
  - Bytecode storage and decoding
  - Instruction parsing
  - Program counter management
  - JUMPDEST analysis
- **Key Type**: `Contract<'ctx>`
- **Why Needed**: Represents deployed contract code

### Infrastructure Crates

#### **cbse-build** (Forge Integration)
- **Purpose**: Solidity compilation
- **Responsibilities**:
  - Executing `forge build`
  - Parsing `out/` directory
  - Extracting bytecode, ABI, storage layout
  - Test contract discovery
- **Key Function**: `run_forge_build()`
- **Why Needed**: Converts Solidity to bytecode CBSE can execute

#### **cbse-remote** (SSH Execution)
- **Purpose**: Remote execution orchestration
- **Responsibilities**:
  - SSH connection management (password auth)
  - SFTP file upload/download
  - Job artifact serialization
  - Remote command execution
- **Key Structures**:
  - `SshConnection`: SSH session wrapper
  - `RemoteExecutor`: Job orchestration
  - `JobArtifact`: Serialized test data
  - `JobResult`: Execution results
- **Why Needed**: Enables cloud execution without full Forge installation on remote

#### **cbse-traces** (Execution Traces)
- **Purpose**: Trace collection and rendering
- **Responsibilities**:
  - Recording CALL, SLOAD, SSTORE operations
  - Panic detection (0x01, 0x11, 0x12, etc.)
  - Human-readable trace formatting
  - Indentation for nested calls
- **Key Types**: `TraceElement`, `CallContext`
- **Why Needed**: Helps users understand why tests fail

#### **cbse-ui** (User Interface)
- **Purpose**: Terminal output formatting
- **Responsibilities**:
  - Progress indicators
  - Color-coded output
  - Test result summaries
  - ASCII art headers
- **Why Needed**: Professional user experience

### Helper Crates

#### **cbse-calldata** (Calldata Construction)
- **Purpose**: Function call encoding
- **Responsibilities**:
  - ABI encoding for function calls
  - Symbolic argument generation
  - Selector calculation (keccak256)
- **Why Needed**: Creates symbolic inputs for functions

#### **cbse-env** (Environment Variables)
- **Purpose**: EVM context (block, tx, msg)
- **Responsibilities**:
  - `msg.sender`, `msg.value`, `block.timestamp`
  - Address and value symbolic values
  - Cheatcode state (pranks, deals)
- **Why Needed**: Models blockchain environment

#### **cbse-cheatcodes** (Foundry Cheatcodes)
- **Purpose**: vm.* function support
- **Responsibilities**:
  - `vm.assume()`, `vm.prank()`, `vm.deal()`
  - Storage manipulation
  - Assertion helpers
- **Why Needed**: Foundry test compatibility

#### **cbse-exceptions** (Error Handling)
- **Purpose**: Unified error types
- **Responsibilities**:
  - `CbseException` enum
  - `CbseResult<T>` type
  - Error propagation
- **Why Needed**: Consistent error handling across crates

#### **cbse-hashes** (Cryptographic Hashing)
- **Purpose**: keccak256 implementation
- **Responsibilities**:
  - Function selector calculation
  - Storage slot hashing
  - Event signature hashing
- **Why Needed**: EVM uses keccak256 extensively

#### **cbse-constants** (EVM Constants)
- **Purpose**: Magic numbers and limits
- **Responsibilities**:
  - Max uint values (2^256-1)
  - Gas limits
  - Panic codes (0x01, 0x11, etc.)
- **Why Needed**: Centralized constant definitions

#### **cbse-assertions** (Test Assertions)
- **Purpose**: Assertion detection
- **Responsibilities**:
  - `assert()` opcode patterns
  - `require()` detection
  - Custom error handling
- **Why Needed**: Recognizes test failures

#### **cbse-logs** (Event Logging)
- **Purpose**: LOG0-LOG4 handling
- **Responsibilities**:
  - Event emission tracking
  - Topic extraction
  - Console.log integration
- **Why Needed**: Captures contract events

#### **cbse-console** (Console.log)
- **Purpose**: Debugging output
- **Responsibilities**:
  - `console.log()` function support
  - String/number formatting
  - Foundry console compatibility
- **Why Needed**: Developer-friendly debugging

#### **cbse-mapper** (Source Mapping)
- **Purpose**: Bytecode to source line mapping
- **Responsibilities**:
  - Parsing Solidity source maps
  - PC to line number translation
  - Error location reporting
- **Why Needed**: Shows where in Solidity code errors occur

#### **cbse-utils** (Utilities)
- **Purpose**: Common helper functions
- **Responsibilities**:
  - Byte manipulation
  - Hex encoding/decoding
  - UID generation
- **Why Needed**: Shared functionality

#### **cbse-solvers** (Multi-Solver Support)
- **Purpose**: Solver abstraction layer
- **Responsibilities**:
  - Z3 wrapper
  - Future: Bitwuzla, CVC5, Yices support
  - Solver-agnostic API
- **Why Needed**: Allows swapping SMT solvers

#### **cbse-processes** (Process Management)
- **Purpose**: External process execution
- **Responsibilities**:
  - Forge subprocess spawning
  - Output capture
  - Timeout enforcement
- **Why Needed**: Runs external tools safely

#### **cbse-memtrace** (Memory Tracing)
- **Purpose**: Memory access tracking
- **Responsibilities**:
  - MLOAD/MSTORE recording
  - Memory growth tracking
  - Optimization hints
- **Why Needed**: Performance analysis and debugging

#### **cbse-flamegraphs** (Performance Profiling)
- **Purpose**: Execution profiling
- **Responsibilities**:
  - Opcode frequency counting
  - Execution time tracking
  - Flamegraph generation
- **Why Needed**: Identifies bottlenecks

---

## Execution Flow

### Phase 1: Compilation (Always Local)

```
┌─────────────────────────────────────────────────────────────┐
│ STEP 1: Contract Compilation (cbse-build)                   │
└─────────────────────────────────────────────────────────────┘

User runs: cbse --function "test"
    │
    ├─→ cbse-config parses CLI arguments
    │
    ├─→ cbse-build executes: forge build --ast --extra-output storageLayout
    │
    ├─→ Forge compiles Solidity contracts:
    │       test/Counter.t.sol → out/Counter.t.sol/CounterTest.json
    │       src/Counter.sol    → out/Counter.sol/Counter.json
    │
    └─→ cbse-build parses JSON artifacts:
            {
              "abi": [...],
              "bytecode": {...},
              "deployedBytecode": {
                "object": "0x6080604052...",  ← This is what we execute
                "sourceMap": "..."
              },
              "storageLayout": {...},
              "metadata": {...}
            }

Output: BuildArtifacts struct with all contract data
```

### Phase 2A: Local Execution

```
┌─────────────────────────────────────────────────────────────┐
│ STEP 2A: Local Symbolic Execution                           │
└─────────────────────────────────────────────────────────────┘

For each test function discovered:

1. Initialize SEVM
   ├─→ cbse-solver creates Z3 context
   ├─→ cbse-sevm creates initial State
   └─→ cbse-contract loads bytecode

2. Deploy Test Contract
   ├─→ Create symbolic address for test contract
   ├─→ Store bytecode in contract storage
   └─→ Set up symbolic environment (msg.sender, block.timestamp)

3. Deploy Target Contracts
   ├─→ If test needs Counter.sol, deploy it
   ├─→ Run constructor with symbolic/concrete args
   └─→ Store deployed bytecode at contract address

4. Execute Test Function
   ├─→ cbse-calldata builds function call:
   │       testFuzz_SetNumber(uint256 x)
   │       → selector: keccak256("testFuzz_SetNumber(uint256)")[:4]
   │       → calldata: [selector][symbolic_x]
   │
   ├─→ cbse-sevm starts execution loop:
   │   │
   │   ├─→ OPCODE: PUSH1 0x80
   │   │   └─→ cbse-bitvec creates symbolic value, pushes to stack
   │   │
   │   ├─→ OPCODE: MSTORE
   │   │   └─→ cbse-bytevec updates memory[offset] = value
   │   │
   │   ├─→ OPCODE: CALLDATALOAD 0x04
   │   │   └─→ Loads symbolic x from calldata
   │   │
   │   ├─→ OPCODE: SSTORE slot=0, value=x
   │   │   └─→ Storage[contract][0] = symbolic x
   │   │
   │   ├─→ OPCODE: JUMPI (conditional branch)
   │   │   └─→ cbse-sevm FORKS execution:
   │   │       ├─→ Path 1: branch_condition == true
   │   │       └─→ Path 2: branch_condition == false
   │   │
   │   ├─→ For each path:
   │   │   ├─→ cbse-solver checks if path_constraints are satisfiable
   │   │   ├─→ If SAT: continue execution
   │   │   └─→ If UNSAT: discard path (infeasible)
   │   │
   │   └─→ OPCODE: REVERT (assertion failure detected)
   │       ├─→ cbse-traces records execution trace
   │       ├─→ cbse-solver extracts counterexample:
   │       │       x = 115792089237316195423570985008687907853269984665640564039457584007913129639935
   │       └─→ Test FAILS
   │
   └─→ cbse-ui displays results:
           ✗ testFuzz_SetNumber(uint256)
           Counterexample: x = 0xfff...fff

5. Aggregate Results
   └─→ Summary: X tests, Y passed, Z failed
```

### Phase 2B: SSH Cloud Execution

```
┌─────────────────────────────────────────────────────────────┐
│ STEP 2B: Remote Symbolic Execution (SSH Mode)               │
└─────────────────────────────────────────────────────────────┘

User runs: cbse --ssh --ssh-host node10@node10 --function "test"
    │
    ├─→ [LOCAL] Steps 1-3 same as local mode (compilation)
    │
    ├─→ [LOCAL] cbse-remote creates JobArtifact:
    │       {
    │         "contracts": [
    │           {
    │             "name": "Counter",
    │             "bytecode": "0x6080...",
    │             "abi": [...]
    │           }
    │         ],
    │         "config": {
    │           "verbosity": 3,
    │           "solver_timeout_ms": 30000,
    │           "loop_bound": 3,
    │           ...
    │         },
    │         "job_id": "uuid-1234-5678",
    │         "timestamp": "2025-11-07T10:30:00Z"
    │       }
    │
    ├─→ [LOCAL] cbse-remote::SshConnection::connect()
    │       ├─→ Prompt for password
    │       ├─→ TCP connection to node10:22
    │       └─→ SSH authentication
    │
    ├─→ [LOCAL] cbse-remote uploads via SFTP:
    │       /tmp/cbse-jobs/uuid-1234-5678/artifact.json
    │
    ├─→ [LOCAL] cbse-remote executes remote command:
    │       /usr/local/bin/cbse --worker-mode \
    │         --input /tmp/cbse-jobs/uuid-1234-5678/artifact.json \
    │         --output /tmp/cbse-jobs/uuid-1234-5678/result.json
    │
    ├─→ [REMOTE] Worker mode starts:
    │   │
    │   ├─→ Deserialize artifact.json
    │   ├─→ Initialize Z3 context
    │   ├─→ Execute tests (same as local Phase 2A, steps 2-4)
    │   ├─→ Serialize results to result.json:
    │   │       {
    │   │         "tests": [
    │   │           {
    │   │             "name": "testFuzz_SetNumber",
    │   │             "passed": false,
    │   │             "trace": "CALL 0x...\nREVERT 0x...",
    │   │             "counterexample": {"x": "0xfff..."}
    │   │           }
    │   │         ],
    │   │         "summary": {...}
    │   │       }
    │   └─→ Exit with code 0 (all pass) or 1 (some fail)
    │
    ├─→ [LOCAL] cbse-remote downloads result.json via SFTP
    │
    ├─→ [LOCAL] cbse-remote cleans up:
    │       rm -rf /tmp/cbse-jobs/uuid-1234-5678/
    │
    └─→ [LOCAL] cbse-ui displays results (same as local mode)
```

---

## Data Flow Diagrams

### Local Mode Data Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                     LOCAL MODE DATA FLOW                          │
└──────────────────────────────────────────────────────────────────┘

   Solidity Source
        │
        │ (cbse-build)
        ▼
   BuildArtifacts
    ├─ bytecode: Vec<u8>
    ├─ abi: Vec<AbiItem>
    └─ storage: StorageLayout
        │
        │ (main.rs)
        ▼
   For each test:
        │
        │ (cbse-sevm)
        ▼
   SEVM Initialization
    ├─ Context (Z3)
    ├─ State (stack, memory, storage)
    └─ Contract (bytecode)
        │
        │ (cbse-calldata)
        ▼
   Function Call
    ├─ selector: [u8; 4]
    └─ args: Vec<CbseBitVec>
        │
        │ (cbse-sevm opcodes)
        ▼
   Execution Loop ──┐
    │               │ (branch)
    ├─ Path 1       │
    │  └─ constraints: Vec<Z3Bool>
    │                │
    └─ Path 2       │
       └─ constraints: Vec<Z3Bool>
                     │
                     │ (cbse-solver)
                     ▼
                 SAT Check
                  ├─ SAT: continue
                  └─ UNSAT: prune
                     │
                     │ (cbse-traces)
                     ▼
                 Trace Recording
                  ├─ CALL
                  ├─ SLOAD
                  ├─ SSTORE
                  └─ REVERT
                     │
                     │ (main.rs)
                     ▼
                 TestResult
                  ├─ passed: bool
                  ├─ trace: String
                  └─ counterexample: Option<Model>
                     │
                     │ (cbse-ui)
                     ▼
                 Terminal Output
```

### SSH Cloud Mode Data Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                   SSH CLOUD MODE DATA FLOW                        │
└──────────────────────────────────────────────────────────────────┘

   ┌─────────────┐                           ┌─────────────┐
   │   LOCAL     │                           │   REMOTE    │
   └─────────────┘                           └─────────────┘

   Solidity Source
        │
        │ (cbse-build - LOCAL)
        ▼
   BuildArtifacts ──────┐
        │               │
        │ (cbse-remote) │
        ▼               │
   JobArtifact          │
    ├─ contracts []     │
    ├─ config           │
    └─ metadata         │
        │               │
        │ serialize     │
        ▼               │
   artifact.json        │
        │               │
        │ SFTP upload   │
        ├───────────────┼───────────────────→ artifact.json
        │               │                          │
        │ SSH exec      │                          │
        ├───────────────┼──→ cbse --worker-mode ←──┘
        │               │           │
        │               │           │ (cbse-sevm - REMOTE)
        │               │           ▼
        │               │      Execution Loop
        │               │       (same as local)
        │               │           │
        │               │           │ (cbse-solver - REMOTE)
        │               │           ▼
        │               │      Symbolic Execution
        │               │           │
        │               │           │ serialize
        │               │           ▼
        │               │      result.json
        │ SFTP download │           │
        │ ←─────────────┼───────────┘
        ▼               │
   JobResult            │
    ├─ tests []         │
    └─ summary          │
        │               │
        │ (cbse-ui)     │
        ▼               │
   Terminal Output      │
```

### Worker Mode JSON Structure

```json
// artifact.json (uploaded to remote)
{
  "contracts": [
    {
      "name": "Counter",
      "bytecode": "0x608060405234801561001057600080fd5b50...",
      "abi": [
        {
          "type": "function",
          "name": "increment",
          "inputs": [],
          "outputs": []
        }
      ],
      "test_functions": ["test_Increment", "testFuzz_SetNumber"]
    }
  ],
  "config": {
    "verbosity": 3,
    "debug": false,
    "print_setup_states": false,
    "print_traces": true,
    "solver_timeout_ms": 30000,
    "solver_max_memory": 8192,
    "loop_bound": 3,
    "width_bound": 5,
    "depth_bound": 100,
    "array_lengths": null,
    "symbolic_storage": false,
    "symbolic_msg_sender": false
  },
  "job_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "timestamp": "2025-11-07T10:30:15Z"
}

// result.json (downloaded from remote)
{
  "tests": [
    {
      "name": "CounterTest::test_Increment()",
      "passed": true,
      "gas_used": 0,
      "return_data": "",
      "trace": "CALL 0xabcd1234::0x273a7c12() (caller: 0x12345678)\n↩ RETURN 0x",
      "counterexample": null
    },
    {
      "name": "CounterTest::testFuzz_SetNumber(uint256)",
      "passed": false,
      "gas_used": 0,
      "return_data": "0x4e487b710000000000000000000000000000000000000000000000000000000000000011",
      "trace": "CALL 0xabcd1234::0xabc12345() (caller: 0x12345678)\n↩ REVERT 0x4e487b71...",
      "counterexample": {
        "x": "115792089237316195423570985008687907853269984665640564039457584007913129639935"
      }
    }
  ],
  "summary": {
    "total": 2,
    "passed": 1,
    "failed": 1,
    "execution_time_ms": 4523
  }
}
```

---

## User Guide

### Installation

#### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Foundry (for Forge)
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Install Z3 SMT Solver
# macOS:
brew install z3

# Ubuntu/Debian:
sudo apt-get install z3 libz3-dev

# Fedora:
sudo dnf install z3 z3-devel
```

#### Build CBSE
```bash
git clone https://github.com/leojay-net/FM-Rust-Cloud.git
cd FM-rust-cloud
cargo build --release
cargo install --path crates/cbse
```

### Local Execution

#### Basic Usage
```bash
# Navigate to your Foundry project
cd my-project

# Run all tests
cbse --function "test"

# Run specific test
cbse --function "testFuzz_SetNumber"

# Run with verbose output
cbse --function "test" -vvv

# Run with debugging
cbse --function "test" --debug --print-traces
```

#### Configuration Options
```bash
# Solver settings
cbse --function "test" \
  --solver-timeout-ms 60000 \
  --solver-max-memory 16384

# Exploration bounds
cbse --function "test" \
  --loop 5 \
  --width 10 \
  --depth 200

# Symbolic configuration
cbse --function "test" \
  --symbolic-storage \
  --symbolic-msg-sender

# Array lengths
cbse --function "test" \
  --array-lengths "MyArray=5,OtherArray=10"
```

### SSH Cloud Execution

#### Setup Remote Server

```bash
# 1. SSH into your remote server
ssh user@remote-server

# 2. Install dependencies
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev z3 libz3-dev clang

# 3. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 4. Clone and install CBSE
git clone https://github.com/leojay-net/FM-Rust-Cloud.git
cd FM-rust-cloud
cargo install --path crates/cbse

# 5. Create symlink (optional)
sudo ln -s ~/.cargo/bin/cbse /usr/local/bin/cbse

# 6. Verify installation
cbse --version
```

#### Run Tests on Remote Server

```bash
# From your local machine
cd my-project

# Run on remote server
cbse --ssh --ssh-host user@remote-server --function "test"

# With custom port
cbse --ssh --ssh-host user@remote-server --ssh-port 2222 --function "test"

# With verbose output
cbse --ssh --ssh-host user@remote-server --function "test" -vvv

# All configuration options work with SSH
cbse --ssh --ssh-host user@remote-server \
  --function "test" \
  --solver-timeout-ms 120000 \
  --loop 10 \
  --debug
```

#### How SSH Mode Works

1. **Local Compilation**: Forge builds contracts on your machine
2. **Artifact Upload**: CBSE uploads bytecode + config to remote via SFTP
3. **Remote Execution**: Remote CBSE runs symbolic execution using Z3
4. **Result Download**: Results are downloaded and displayed locally
5. **Cleanup**: Temporary files removed from remote server

**Advantages**:
- No need to sync source code to remote
- No need to install Forge on remote
- Only bytecode is transferred (small payload)
- Full config control from local CLI
- Results displayed locally with same UI

#### Monitoring Remote Execution

On the remote server, you can monitor CBSE execution:

```bash
# Monitor active processes
watch -n 1 'ps aux | grep cbse'

# Monitor job directories
watch -n 1 'ls -lh /tmp/cbse-jobs/'

# View logs (if CBSE is verbose)
tail -f /tmp/cbse-jobs/*/output.log
```

### Advanced Usage

#### Custom Test Patterns
```bash
# Run all fuzz tests
cbse --function "testFuzz"

# Run specific contract's tests
cbse --function "test" --contract "CounterTest"

# Run invariant tests
cbse --function "invariant"
```

#### Performance Tuning
```bash
# Fast mode (lower bounds)
cbse --function "test" --loop 2 --width 3 --depth 50

# Deep analysis (higher bounds)
cbse --function "test" --loop 10 --width 20 --depth 500

# Timeout after 2 minutes per test
cbse --function "test" --solver-timeout-ms 120000
```

#### Debugging Failed Tests
```bash
# Show full execution traces
cbse --function "testFuzz_SetNumber" \
  -vvvvvv \
  --print-traces \
  --print-setup-states \
  --debug

# This will show:
# - Constructor execution
# - Setup function execution
# - Test function execution with symbolic values
# - All storage reads/writes
# - Full call stack
# - Panic codes
```

### Example Workflow

#### Scenario: Finding a Bug in SimpleVault

```solidity
// src/SimpleVault.sol
contract SimpleVault {
    mapping(address => uint256) public balances;

    function deposit() external payable {
        balances[msg.sender] += msg.value;
    }

    function withdraw(uint256 amount) external {
        require(balances[msg.sender] >= amount, "Insufficient balance");
        balances[msg.sender] -= amount;  // BUG: Should happen AFTER transfer
        payable(msg.sender).transfer(amount);
    }
}

// test/SimpleVault.t.sol
contract SimpleVaultTest is Test {
    SimpleVault vault;

    function setUp() public {
        vault = new SimpleVault();
    }

    function testWithdraw(uint256 depositAmount, uint256 withdrawAmount) public {
        vm.assume(depositAmount > 0);
        vm.assume(withdrawAmount > 0);
        
        vault.deposit{value: depositAmount}();
        vault.withdraw(withdrawAmount);
        
        // This should hold, but CBSE will find a counterexample
        assert(address(vault).balance >= 0);
    }
}
```

#### Local Execution
```bash
$ cbse --function "testWithdraw" -vvv

   ╔═══════════════════════════════════════════╗
   ║  CBSE - Complete Blockchain Symbolic     ║
   ║         Executor (Rust Edition)           ║
   ╚═══════════════════════════════════════════╝

  Executing testWithdraw(uint256,uint256)
    
    ✗ Counterexample found!
    
    Symbolic inputs:
      depositAmount = 100
      withdrawAmount = 200
    
    Trace:
    CALL SimpleVault::deposit() value=100
      SSTORE balances[caller] = 100
    ↩ RETURN
    
    CALL SimpleVault::withdraw(200)
      SLOAD balances[caller] → 100
      ✓ require(100 >= 200)  ← FAILS
    ↩ REVERT "Insufficient balance"

Summary: 1 test, 0 passed, 1 failed
```

#### SSH Execution (Same Contract)
```bash
$ cbse --ssh --ssh-host compute-node --function "testWithdraw" -vvv

Running in SSH mode (remote execution)
Enter SSH password: ****
🔌 Connecting to compute-node:22...
✅ SSH connection established
📤 Uploading artifacts...
⚙️  Executing CBSE on remote node...

📋 Remote output:
   ╔═══════════════════════════════════════════╗
   ║  CBSE - Complete Blockchain Symbolic     ║
   ║         Executor (Rust Edition)           ║
   ╚═══════════════════════════════════════════╝

  Executing testWithdraw(uint256,uint256)
    
    ✗ Counterexample found!
    
    Symbolic inputs:
      depositAmount = 100
      withdrawAmount = 200
    
    [... same trace as local ...]

📥 Downloading results...
✅ Remote execution complete in 3.45s

Summary: 1 test, 0 passed, 1 failed
```

**Identical results!** The only difference is where Z3 solver runs.

---

## Performance Comparison

| Aspect | Local Mode | SSH Cloud Mode |
|--------|-----------|----------------|
| **Compilation Speed** | Fast (local CPU) | Fast (local CPU) |
| **Network Overhead** | None | Upload: ~1-10KB/test<br>Download: ~5-50KB/test |
| **Execution Speed** | Depends on local CPU/RAM | Depends on remote CPU/RAM |
| **Solver Performance** | Local Z3 | Remote Z3 (potentially more RAM) |
| **Total Time (small contract)** | ~2-5s | ~5-10s (network + exec) |
| **Total Time (large contract)** | ~30-120s | ~20-60s (faster remote CPU) |
| **Best For** | Dev/debug, quick tests | CI/CD, heavy analysis |

---

## Troubleshooting

### Common Issues

#### 1. Z3 Not Found
```
Error: Z3 library not found
```

**Solution**:
```bash
# macOS
brew install z3

# Ubuntu
sudo apt-get install z3 libz3-dev

# Set library path if needed
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
```

#### 2. SSH Connection Fails
```
Error: Failed to connect to remote server
```

**Solution**:
```bash
# Test SSH manually
ssh user@remote-server

# Check port
cbse --ssh --ssh-host user@remote-server --ssh-port 22

# Verify remote binary exists
ssh user@remote-server 'which cbse'
```

#### 3. Tests Timeout
```
Error: Solver timeout after 30000ms
```

**Solution**:
```bash
# Increase timeout
cbse --function "test" --solver-timeout-ms 120000

# Reduce bounds
cbse --function "test" --loop 2 --depth 50
```

#### 4. Out of Memory
```
Error: Z3 memory limit exceeded
```

**Solution**:
```bash
# Increase memory limit
cbse --function "test" --solver-max-memory 16384

# Or use remote server with more RAM
cbse --ssh --ssh-host big-server --function "test"
```

---

## FAQ

**Q: Why Rust instead of Python (like Halmos)?**  
A: Performance. Rust is 10-50x faster for symbolic execution, with lower memory usage and better parallelization.

**Q: Can I run multiple tests in parallel?**  
A: Currently, tests run sequentially. Parallel execution is planned for v0.2.0.

**Q: Does CBSE support all Foundry cheatcodes?**  
A: Most common ones (vm.assume, vm.prank, vm.deal). Full list in `cbse-cheatcodes/src/lib.rs`.

**Q: Can I use multiple remote servers?**  
A: Not yet. Multi-node parallel execution is planned for v0.3.0.

**Q: How do I contribute?**  
A: See `CONTRIBUTING.md`. We welcome PRs for new cheatcodes, optimizations, and bug fixes!

**Q: What's the license?**  
A: MIT License (same as Halmos).

---

## Summary

CBSE provides **two execution modes** for Ethereum symbolic execution:

1. **Local Mode**: Everything runs on your machine
   - Best for: Development, debugging, small contracts
   - Requires: Rust, Forge, Z3

2. **SSH Cloud Mode**: Compile locally, execute remotely
   - Best for: CI/CD, large contracts, resource-limited local machines
   - Requires: Rust + Forge locally, CBSE + Z3 on remote

Both modes use the **same core engine** (cbse-sevm + Z3) and produce **identical results**. The architecture is modular, with 25+ crates handling different aspects:

- **Compilation**: cbse-build (Forge integration)
- **Execution**: cbse-sevm (EVM interpreter)
- **Solving**: cbse-solver (Z3 wrapper)
- **Data**: cbse-bitvec, cbse-bytevec (symbolic types)
- **Infrastructure**: cbse-remote (SSH), cbse-ui (output)

This design allows **local-first development** with **optional cloud offloading** for heavy workloads.
