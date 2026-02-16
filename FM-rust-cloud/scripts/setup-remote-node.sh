#!/bin/bash
# SPDX-License-Identifier: AGPL-3.0
#
# Setup script for CBSE remote execution node
#
# This script:
# 1. Builds CBSE in release mode
# 2. Uploads the binary to the remote node
# 3. Installs it in /usr/local/bin
# 4. Creates working directory
# 5. Tests the connection

set -e

# Configuration
REMOTE_HOST="${1:-node10@node10}"
REMOTE_BINARY_PATH="/usr/local/bin/cbse"
REMOTE_WORKDIR="/tmp/cbse-jobs"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "╔════════════════════════════════════════════╗"
echo "║  CBSE Remote Node Setup                    ║"
echo "╚════════════════════════════════════════════╝"
echo ""
echo "Remote host: $REMOTE_HOST"
echo "Remote binary: $REMOTE_BINARY_PATH"
echo "Working directory: $REMOTE_WORKDIR"
echo "Project root: $PROJECT_ROOT"
echo ""

# Step 1: Build release binary
echo "📦 Building CBSE in release mode..."
cd "$PROJECT_ROOT"
cargo build --release

if [ ! -f "target/release/cbse" ]; then
    echo "❌ Error: Release binary not found at target/release/cbse"
    exit 1
fi

BINARY_SIZE=$(ls -lh target/release/cbse | awk '{print $5}')
echo "✅ Binary built successfully (size: $BINARY_SIZE)"
echo ""

# Step 2: Test SSH connection
echo "🔍 Testing SSH connection to $REMOTE_HOST..."
if ! ssh -o ConnectTimeout=5 "$REMOTE_HOST" "echo 'SSH connection successful'" > /dev/null 2>&1; then
    echo "❌ Error: Cannot connect to $REMOTE_HOST"
    echo "   Please verify:"
    echo "   1. SSH server is running"
    echo "   2. Tailscale is connected (if using Tailscale)"
    echo "   3. Hostname and username are correct"
    exit 1
fi
echo "✅ SSH connection verified"
echo ""

# Step 3: Upload binary
echo "📤 Uploading CBSE binary to remote node..."
scp target/release/cbse "${REMOTE_HOST}:/tmp/cbse"
echo "✅ Binary uploaded to /tmp/cbse"
echo ""

# Step 4: Install and configure on remote
echo "🔧 Installing CBSE on remote node..."
ssh -tt "$REMOTE_HOST" bash <<EOF
set -e

# Move binary to final location (may require sudo)
if [ -w "/usr/local/bin" ]; then
    mv /tmp/cbse $REMOTE_BINARY_PATH
else
    echo "Moving binary to $REMOTE_BINARY_PATH (requires sudo)"
    sudo mv /tmp/cbse $REMOTE_BINARY_PATH
fi

# Make executable
if [ -w "$REMOTE_BINARY_PATH" ]; then
    chmod +x $REMOTE_BINARY_PATH
else
    sudo chmod +x $REMOTE_BINARY_PATH
fi

# Create working directory
mkdir -p $REMOTE_WORKDIR
chmod 755 $REMOTE_WORKDIR

echo "✅ CBSE installed at $REMOTE_BINARY_PATH"

# Verify installation
if [ -x "$REMOTE_BINARY_PATH" ]; then
    echo "✅ Binary is executable"
    VERSION=\$($REMOTE_BINARY_PATH --version)
    echo "   Version: \$VERSION"
else
    echo "❌ Error: Binary is not executable"
    exit 1
fi

# Check Z3 solver
if command -v z3 > /dev/null 2>&1; then
    Z3_VERSION=\$(z3 --version | head -1)
    echo "✅ Z3 solver found: \$Z3_VERSION"
else
    echo "⚠️  Warning: Z3 solver not found in PATH"
    echo "   CBSE may not work correctly without Z3"
    echo "   Install Z3: https://github.com/Z3Prover/z3"
fi

echo ""
echo "Remote node configuration complete!"
EOF

echo ""
echo "✅ Remote installation complete!"
echo ""

# Step 5: Test CBSE connection
echo "🧪 Testing CBSE remote execution..."

# Create temporary test contract directory
TEST_DIR="$PROJECT_ROOT/../test-contract"

if [ -d "$TEST_DIR" ]; then
    cd "$TEST_DIR"
    
    # Test connection using CBSE
    if "$PROJECT_ROOT/target/debug/cbse" --ssh --ssh-host "$REMOTE_HOST" --ssh-test; then
        echo "✅ CBSE remote connection test passed!"
    else
        echo "⚠️  CBSE remote connection test failed"
        echo "   The binary is installed but connection test failed"
    fi
else
    echo "ℹ️  Test contract directory not found, skipping connection test"
    echo "   You can manually test with:"
    echo "   cbse --ssh --ssh-host $REMOTE_HOST --ssh-test"
fi

echo ""
echo "╔════════════════════════════════════════════╗"
echo "║  Setup Complete! 🎉                        ║"
echo "╚════════════════════════════════════════════╝"
echo ""
echo "Next steps:"
echo "  1. Test connection:"
echo "     cbse --ssh --ssh-host $REMOTE_HOST --ssh-test"
echo ""
echo "  2. Run tests remotely:"
echo "     cbse test/Counter.t.sol --ssh --ssh-host $REMOTE_HOST"
echo ""
echo "  3. Compare local vs remote performance:"
echo "     time cbse test/Counter.t.sol                              # Local"
echo "     time cbse test/Counter.t.sol --ssh --ssh-host $REMOTE_HOST  # Remote"
echo ""
