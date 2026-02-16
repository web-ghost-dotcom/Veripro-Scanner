# CBSE Coordinator AWS Deployment Guide

This guide covers deploying the CBSE Coordinator to AWS EC2.

## Architecture Overview

```
┌─────────────────┐     HTTPS      ┌─────────────────┐
│  Vercel         │───────────────▶│  AWS EC2        │
│  (Frontend)     │                │  (Coordinator)  │
└─────────────────┘                │                 │
                                   │  - cbse-coord   │
                                   │  - cbse binary  │
                                   │  - forge        │
                                   │  - z3           │
                                   └─────────────────┘
```

## Prerequisites

- AWS Account with EC2 access
- SSH key pair for EC2
- Domain name (optional, for HTTPS)

---

## Option 1: EC2 Deployment (Recommended)

### Step 1: Launch EC2 Instance

1. Go to AWS Console → EC2 → Launch Instance
2. Choose settings:
   - **Name**: `veripro-coordinator`
   - **AMI**: Ubuntu 22.04 LTS (or Amazon Linux 2023)
   - **Instance type**: `t3.medium` (2 vCPU, 4GB RAM) minimum
   - **Key pair**: Create or select existing
   - **Security group**: 
     - SSH (22) from your IP
     - Custom TCP (3001) from anywhere (or your Vercel IP range)
     - HTTPS (443) from anywhere (if using domain)
   - **Storage**: 30GB gp3

3. Launch the instance

### Step 2: Connect to Instance

```bash
ssh -i your-key.pem ubuntu@<EC2-PUBLIC-IP>
```

### Step 3: Install Dependencies

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install build essentials
sudo apt install -y build-essential pkg-config libssl-dev git curl


# Install Z3 SMT Solver
sudo apt install -y z3 libz3-dev

 # Install clang and libclang-dev (required for z3-sys bindgen)
sudo apt install -y clang libclang-dev

# Also ensure you have the full GCC toolchain
sudo apt install -y gcc g++ libc6-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
source ~/.bashrc
foundryup

# Verify installations
rustc --version
forge --version
z3 --version
```

### Step 4: Clone and Build

```bash
# Clone repository
git clone https://github.com/leojay-net/VERIPRO.git
cd VERIPRO/FM-rust-cloud

# Build in release mode
cargo build --release

# Verify binary
./target/release/cbse-coordinator --help 2>/dev/null || echo "Binary built successfully"
```

### Step 5: Setup forge-std

```bash
# Create forge-std directory
mkdir -p ~/forge-std
cd ~/forge-std
forge init --no-git temp
mv temp/lib/forge-std/* .
rm -rf temp

# Set environment variable
echo 'export FORGE_STD_PATH=~/forge-std' >> ~/.bashrc
source ~/.bashrc
```

### Step 6: Configure Environment

```bash
# Create environment file
cat > ~/VERIPRO/FM-rust-cloud/.env << 'EOF'
# Prover private key (use your actual key)
PROVER_PRIVATE_KEY=your_private_key_here

# Path to forge-std
FORGE_STD_PATH=/home/ubuntu/forge-std

# CBSE binary path
CBSE_BINARY=/home/ubuntu/VERIPRO/FM-rust-cloud/target/release/cbse
EOF

# Load environment
source ~/VERIPRO/FM-rust-cloud/.env
```

### Step 7: Create Systemd Service

```bash
sudo tee /etc/systemd/system/cbse-coordinator.service << 'EOF'
[Unit]
Description=CBSE Coordinator
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/VERIPRO/FM-rust-cloud
Environment=PROVER_PRIVATE_KEY=your_private_key_here
Environment=FORGE_STD_PATH=/home/ubuntu/forge-std
Environment=CBSE_BINARY=/home/ubuntu/VERIPRO/FM-rust-cloud/target/release/cbse
Environment=RUST_LOG=info
ExecStart=/home/ubuntu/VERIPRO/FM-rust-cloud/target/release/cbse-coordinator
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Reload and start
sudo systemctl daemon-reload
sudo systemctl enable cbse-coordinator
sudo systemctl start cbse-coordinator

# Check status
sudo systemctl status cbse-coordinator
```

### Step 8: Setup Nginx Reverse Proxy (Optional but Recommended)

```bash
# Install nginx
sudo apt install -y nginx certbot python3-certbot-nginx

# Create nginx config
sudo tee /etc/nginx/sites-available/cbse-coordinator << 'EOF'
server {
    listen 80;
    server_name your-domain.com;  # Or use EC2 public IP

    location / {
        proxy_pass http://127.0.0.1:3001;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
        
        # Increase timeout for long verifications
        proxy_read_timeout 300s;
        proxy_connect_timeout 75s;
    }
}
EOF

# Enable site
sudo ln -s /etc/nginx/sites-available/cbse-coordinator /etc/nginx/sites-enabled/
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t
sudo systemctl restart nginx
```

### Step 9: Setup SSL with Let's Encrypt (If using domain)

```bash
# Get SSL certificate
sudo certbot --nginx -d your-domain.com

# Auto-renewal is configured automatically
```

### Step 10: Test the Deployment

```bash
# Test health endpoint
curl http://localhost:3001/health

# From your local machine
curl http://<EC2-PUBLIC-IP>:3001/health

# Or if using nginx
curl https://your-domain.com/health
```

---

## Option 2: Docker Deployment

### Dockerfile

Create `FM-rust-cloud/Dockerfile`:

```dockerfile
FROM rust:1.75-bookworm AS builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    z3 libz3-dev pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build release binary
RUN cargo build --release --bin cbse-coordinator
RUN cargo build --release --bin cbse

FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    z3 libz3-4 ca-certificates curl git \
    && rm -rf /var/lib/apt/lists/*

# Install Foundry
RUN curl -L https://foundry.paradigm.xyz | bash
ENV PATH="/root/.foundry/bin:${PATH}"
RUN foundryup

# Setup forge-std
RUN mkdir -p /forge-std && \
    cd /forge-std && \
    forge init --no-git temp && \
    mv temp/lib/forge-std/* . && \
    rm -rf temp

# Copy binaries
COPY --from=builder /app/target/release/cbse-coordinator /usr/local/bin/
COPY --from=builder /app/target/release/cbse /usr/local/bin/

# Environment
ENV FORGE_STD_PATH=/forge-std
ENV CBSE_BINARY=/usr/local/bin/cbse
ENV RUST_LOG=info

EXPOSE 3001

CMD ["cbse-coordinator"]
```

### Build and Run

```bash
# Build image
docker build -t cbse-coordinator .

# Run container
docker run -d \
  --name cbse-coordinator \
  -p 3001:3001 \
  -e PROVER_PRIVATE_KEY=your_private_key \
  cbse-coordinator
```

### Deploy to AWS ECR + ECS

```bash
# Create ECR repository
aws ecr create-repository --repository-name cbse-coordinator

# Login to ECR
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin <ACCOUNT>.dkr.ecr.us-east-1.amazonaws.com

# Tag and push
docker tag cbse-coordinator:latest <ACCOUNT>.dkr.ecr.us-east-1.amazonaws.com/cbse-coordinator:latest
docker push <ACCOUNT>.dkr.ecr.us-east-1.amazonaws.com/cbse-coordinator:latest

# Then create ECS service via AWS Console or CLI
```

---

## Update Frontend Configuration

After deployment, update your Vercel environment:

1. Go to Vercel Dashboard → Your Project → Settings → Environment Variables
2. Add/Update:
   ```
   NEXT_PUBLIC_COORDINATOR_URL=https://your-domain.com
   ```

3. Update `next.config.ts` rewrite destination:
   ```typescript
   destination: 'https://your-domain.com/verify',
   ```

4. Redeploy frontend

---

## Security Considerations

1. **Private Key**: Never commit your `PROVER_PRIVATE_KEY` to git
2. **Firewall**: Only allow port 3001 from Vercel's IP ranges
3. **HTTPS**: Always use SSL in production
4. **Updates**: Keep system and dependencies updated
5. **Monitoring**: Setup CloudWatch or similar for logs

---

## Monitoring Commands

```bash
# View logs
sudo journalctl -u cbse-coordinator -f

# Check status
sudo systemctl status cbse-coordinator

# Restart service
sudo systemctl restart cbse-coordinator

# View resource usage
htop
```

---

## Estimated Costs

| Resource | Specification | Monthly Cost |
|----------|---------------|--------------|
| EC2 t3.medium | 2 vCPU, 4GB RAM | ~$30 |
| EBS Storage | 30GB gp3 | ~$3 |
| Data Transfer | Variable | ~$5-10 |
| **Total** | | **~$40-50/month** |

For lower costs, consider:
- `t3.small` for light usage (~$15/month)
- Spot instances for 60-90% savings
- Reserved instances for long-term (30-40% savings)

---

## Quick Reference

```bash
# SSH to instance
ssh -i your-key.pem ubuntu@<EC2-IP>

# View logs
sudo journalctl -u cbse-coordinator -f

# Restart service
sudo systemctl restart cbse-coordinator

# Update code
cd ~/VERIPRO && git pull && cd FM-rust-cloud && cargo build --release
sudo systemctl restart cbse-coordinator

# Check health
curl http://localhost:3001/health
```
