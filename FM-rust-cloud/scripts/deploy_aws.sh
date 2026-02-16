#!/bin/bash
# Deploy Backend to AWS EC2
# Usage: ./deploy_aws.sh <EC2_HOST> <SSH_KEY_PATH>
# Example: ./deploy_aws.sh ubuntu@3.12.34.56 ~/.ssh/my-key.pem

set -e

REMOTE_HOST=$1
SSH_KEY=$2

if [ -z "$REMOTE_HOST" ] || [ -z "$SSH_KEY" ]; then
    echo "Usage: $0 <USER@HOST> <SSH_KEY_PATH>"
    exit 1
fi

echo "🚀 Starting Deployment to $REMOTE_HOST..."

# 1. Build binaries locally (faster than building on small EC2)
echo "📦 Building Release Binaries..."
cd "$(dirname "$0")/.."
cargo build --release --bin cbse
cargo build --release --bin cbse-coordinator

# 2. Upload binaries
echo "ki Uploading binaries..."
scp -i "$SSH_KEY" target/release/cbse "$REMOTE_HOST":/tmp/
scp -i "$SSH_KEY" target/release/cbse-coordinator "$REMOTE_HOST":/tmp/

# 3. Move and Restart on Remote
echo "🔧 Installing and Restarting Service..."
ssh -i "$SSH_KEY" -t "$REMOTE_HOST" << 'EOF'
    sudo mv /tmp/cbse /usr/local/bin/
    sudo mv /tmp/cbse-coordinator /usr/local/bin/
    sudo chmod +x /usr/local/bin/cbse
    sudo chmod +x /usr/local/bin/cbse-coordinator
    
    # Check if systemd service exists, if so restart it
    if systemctl is-active --quiet cbse-coordinator; then
        echo "Restarting cbse-coordinator service..."
        sudo systemctl restart cbse-coordinator
    else
        echo "⚠️ Service not running. You may need to set up the systemd service file."
        # Attempt to run it manually if service doesn't exist (for testing)
        # nohup /usr/local/bin/cbse-coordinator > coordinator.log 2>&1 &
    fi
    
    echo "✅ Backend updated successfully!"
EOF
