#!/bin/bash
# setup-server.sh
# Run this script ON THE SERVER after cloning the repo to set up everything.

set -e

echo "🚀 Starting CBSE Server Setup..."

# 1. Install System Dependencies
echo "📦 Installing system dependencies..."
sudo apt-get update
sudo apt-get install -y build-essential libssl-dev pkg-config git z3 libz3-dev curl

# 2. Install Rust (if not installed)
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "✅ Rust is already installed"
fi

# 3. Install Foundry (if not installed)
if ! command -v forge &> /dev/null; then
    echo "⚒️ Installing Foundry..."
    curl -L https://foundry.paradigm.xyz | bash
    export PATH="$HOME/.foundry/bin:$PATH"
    foundryup
else
    echo "✅ Foundry is already installed"
fi

# 3.5 Install forge-std globally (for coordinator to use)
echo "📚 Installing forge-std library..."
FORGE_STD_PATH="$HOME/.foundry/forge-std"
if [ ! -d "$FORGE_STD_PATH" ]; then
    mkdir -p "$HOME/.foundry"
    git clone https://github.com/foundry-rs/forge-std "$FORGE_STD_PATH"
    echo "✅ forge-std installed to $FORGE_STD_PATH"
else
    echo "✅ forge-std already exists at $FORGE_STD_PATH"
    # Optional: Update it
    # cd "$FORGE_STD_PATH" && git pull
fi

# 4. Build Project
echo "🔨 Building project (this may take a while)..."
# Navigate to project root (assuming script is in scripts/)
cd "$(dirname "$0")/.."
cargo build --release --bin cbse
cargo build --release --bin cbse-coordinator

# 5. Install Binaries
echo "📥 Installing binaries to /usr/local/bin..."
sudo cp target/release/cbse /usr/local/bin/
sudo cp target/release/cbse-coordinator /usr/local/bin/
sudo chmod +x /usr/local/bin/cbse
sudo chmod +x /usr/local/bin/cbse-coordinator

# 6. Setup Systemd Service
echo "⚙️ Configuring Systemd service..."
SERVICE_FILE="/etc/systemd/system/cbse-coordinator.service"

# Generate service file content
cat <<EOF | sudo tee "$SERVICE_FILE"
[Unit]
Description=CBSE Coordinator
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME
Environment=CBSE_BINARY=/usr/local/bin/cbse
Environment=PATH=$HOME/.cargo/bin:$HOME/.foundry/bin:/usr/local/bin:/usr/bin:/bin
ExecStart=/usr/local/bin/cbse-coordinator
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOF

# Reload and Start Service
sudo systemctl daemon-reload
sudo systemctl enable cbse-coordinator
sudo systemctl restart cbse-coordinator

echo "✅ Setup Complete!"
echo "Service status:"
sudo systemctl status cbse-coordinator --no-pager
