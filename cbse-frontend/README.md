# VeriPro Frontend

The web interface for VeriPro—an AI-powered Smart Contract Security Scanner and formal verification platform.

## Overview

VeriPro Frontend provides an intuitive interface for:
- **AI Agent Scanning**: Automatically detect vulnerabilities in smart contracts.
- **Formal Verification**: Mathematically prove security properties.
- **Managing Projects**: Organize your audits and scans.
- **On-Chain Attestations**: Publish security proofs to the blockchain.

## Tech Stack

- **Framework**: Next.js 16
- **UI**: React 19, Tailwind CSS
- **Animation**: Framer Motion
- **Wallet**: RainbowKit, wagmi, viem
- **3D**: React Three Fiber (landing page)

## Prerequisites

- Node.js 18+
- npm or yarn

## Installation

```bash
# Install dependencies
npm install

# Create environment file
cp .env.example .env.local
```

## Environment Variables

Create `.env.local` with:

```env
# Google Gemini API (REQUIRED for AI Security features)
GOOGLE_API_KEY=your_gemini_api_key

# WalletConnect Project ID (optional, for wallet connections)
NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID=your_project_id

# Registry contract address (for on-chain attestations on EVM & BNB Chain)
NEXT_PUBLIC_ATTESTATION_REGISTRY_ADDRESS=0xae454F272197b110C28223dbE3e49b4a1c798015
```

## AI Security Agent

The frontend includes a built-in AI Security Agent powered by **Google Gemini**.

### Capabilities
1.  **Autonomous Scanning**: Paste any contract, click "Generate Specs", and the agent will scan for:
    *   **Reentrancy**
    *   **Integer Overflow/Underflow**
    *   **Access Control Violations**
    *   **Front-Running Risks**
2.  **Spec Generation**: The agent writes formal specifications (`foundry-std` invariant tests) to mathematically prove the absence of detected vulnerabilities.
3.  **Chat Interface**: Interact with the agent to refine properties or ask specific security questions.

### Configuration
Ensure `GOOGLE_API_KEY` is set in `.env.local`. The agent uses the `gemini-2.5-flash` model for high-speed analysis.

## Development

```bash
# Start development server (http://localhost:3000)
npm run dev

# Build for production
npm run build

# Start production server
npm start

# Lint code
npm run lint
```

## Project Structure

```
cbse-frontend/
├── app/
│   ├── api/
│   │   └── veripro-ai/      # Gemini AI proxy route
│   ├── app/
│   │   ├── page.tsx         # Dashboard
│   │   └── projects/
│   │       └── [id]/        # Project detail page
│   ├── components/
│   │   ├── SyntaxEditor.tsx # Solidity code editor
│   │   ├── VeriProAI.tsx    # AI assistant panel
│   │   └── ...
│   ├── docs/
│   │   └── page.tsx         # Documentation
│   └── page.tsx             # Landing page
├── public/                   # Static assets
├── next.config.ts           # Next.js config
└── package.json
```

## Configuration

### Coordinator Proxy

The frontend proxies `/api/verify` requests to the CBSE coordinator. Configure the destination in `next.config.ts`:

```typescript
const nextConfig: NextConfig = {
  async rewrites() {
    return [
      {
        source: '/api/verify',
        destination: 'http://127.0.0.1:3001/verify', // Coordinator URL
      },
    ];
  },
};
```

For production, update to your deployed coordinator URL:

```typescript
destination: 'https://your-coordinator.com/verify',
```

## Deployment

### Vercel (Recommended)

```bash
npm install -g vercel
vercel
```

Set environment variables in the Vercel dashboard.

### Docker

```dockerfile
FROM node:18-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:18-alpine AS runner
WORKDIR /app
COPY --from=builder /app/.next ./.next
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/package.json ./
EXPOSE 3000
CMD ["npm", "start"]
```

Build and run:

```bash
docker build -t veripro-frontend .
docker run -p 3000:3000 veripro-frontend
```

### Self-Hosted

```bash
npm run build
npm start
```

Use a reverse proxy (nginx, Caddy) for HTTPS.

## Features

### Syntax-Highlighted Editor

The `SyntaxEditor` component provides:
- Real-time Solidity syntax highlighting
- Line numbers
- Tab indentation
- Scroll synchronization

### AI Specification Assistant

The `VeriProAI` component offers:
- Natural language to specification conversion
- Contract analysis
- One-click code insertion
- Syntax-highlighted code blocks

### Verification Results

Results display:
- Pass/fail status for each property
- Counterexamples for failures
- Execution trace details
- Attestation data

## API Routes

### POST `/api/veripro-ai`

Proxies requests to Google Gemini for AI-powered specification generation.

Request:
```json
{
  "prompt": "Write a test for the transfer function",
  "contractCode": "// Solidity code..."
}
```

Response:
```json
{
  "response": "Here's a specification for your transfer function..."
}
```

### POST `/api/verify` (Proxied)

Sends verification requests to the CBSE coordinator.

Request:
```json
{
  "contract_source": "// Solidity contract...",
  "spec_source": "// Test specification...",
  "contract_name": "MyContract"
}
```

Response:
```json
{
  "job_id": "uuid",
  "status": "Success",
  "message": "Verification completed.",
  "attestation": { ... }
}
```

## Troubleshooting

### AI not working

- Verify `GOOGLE_API_KEY` is set
- Check the API key has Gemini access
- Review browser console for errors

### Verification fails to connect

- Ensure the coordinator is running
- Check `next.config.ts` proxy settings
- Verify network connectivity

### Wallet connection issues

- Set `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID`
- Check browser wallet extension
- Try refreshing the page

## License

MIT
