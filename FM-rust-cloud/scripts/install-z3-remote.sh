#!/bin/bash
# SPDX-License-Identifier: AGPL-3.0
#
# Install Z3 solver on remote node
#
# This script installs Z3 theorem prover which is required for CBSE symbolic execution

set -e

REMOTE_HOST="${1:-node10@node10}"

echo "╔════════════════════════════════════════════╗"
echo "║  Install Z3 Solver on Remote Node         ║"
echo "╚════════════════════════════════════════════╝"
echo ""
echo "Remote host: $REMOTE_HOST"
echo ""

# Check SSH connection
echo "🔍 Testing SSH connection..."
if ! ssh -o ConnectTimeout=5 "$REMOTE_HOST" "echo 'SSH OK'" > /dev/null 2>&1; then
    echo "❌ Error: Cannot connect to $REMOTE_HOST"
    exit 1
fi
echo "✅ SSH connection verified"
echo ""

# Detect OS and install Z3
echo "🔧 Installing Z3 solver on remote node..."
echo "   (You will be prompted for password)"
echo ""

ssh -tt "$REMOTE_HOST" bash <<'EOF'
set -e

echo "Detecting operating system..."

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
    VERSION=$VERSION_ID
elif [ -f /etc/redhat-release ]; then
    OS="rhel"
elif [ "$(uname)" = "Darwin" ]; then
    OS="macos"
else
    OS="unknown"
fi

echo "Detected OS: $OS"
echo ""

case $OS in
    ubuntu|debian)
        echo "📦 Installing Z3 via apt..."
        sudo apt-get update
        sudo apt-get install -y z3
        ;;
        
    fedora|rhel|centos)
        echo "📦 Installing Z3 via dnf/yum..."
        if command -v dnf > /dev/null 2>&1; then
            sudo dnf install -y z3
        else
            sudo yum install -y z3
        fi
        ;;
        
    arch|manjaro)
        echo "📦 Installing Z3 via pacman..."
        sudo pacman -Sy --noconfirm z3
        ;;
        
    macos)
        echo "📦 Installing Z3 via Homebrew..."
        if ! command -v brew > /dev/null 2>&1; then
            echo "❌ Homebrew not found. Please install: https://brew.sh"
            exit 1
        fi
        brew install z3
        ;;
        
    *)
        echo "⚠️  Unknown OS: $OS"
        echo "Please install Z3 manually:"
        echo ""
        echo "Option 1 - Package manager:"
        echo "  Ubuntu/Debian:  sudo apt install z3"
        echo "  Fedora/RHEL:    sudo dnf install z3"
        echo "  Arch:           sudo pacman -S z3"
        echo "  macOS:          brew install z3"
        echo ""
        echo "Option 2 - Download binary:"
        echo "  https://github.com/Z3Prover/z3/releases"
        exit 1
        ;;
esac

echo ""
echo "✅ Z3 installation complete!"
echo ""

# Verify installation
if command -v z3 > /dev/null 2>&1; then
    echo "🧪 Verifying Z3 installation..."
    Z3_VERSION=$(z3 --version | head -1)
    echo "   Version: $Z3_VERSION"
    Z3_PATH=$(which z3)
    echo "   Location: $Z3_PATH"
    echo ""
    echo "✅ Z3 is ready to use!"
else
    echo "⚠️  Warning: z3 command not found in PATH"
    echo "   You may need to log out and back in, or run:"
    echo "   source ~/.bashrc"
    exit 1
fi

EOF

echo ""
echo "╔════════════════════════════════════════════╗"
echo "║  Z3 Installation Complete! 🎉              ║"
echo "╚════════════════════════════════════════════╝"
echo ""
echo "Next steps:"
echo "  1. Verify installation:"
echo "     ssh $REMOTE_HOST 'z3 --version'"
echo ""
echo "  2. Run CBSE setup if not already done:"
echo "     ./scripts/setup-remote-node.sh $REMOTE_HOST"
echo ""
echo "  3. Test CBSE with Z3:"
echo "     cbse test/Counter.t.sol --ssh --ssh-host $REMOTE_HOST"
echo ""
