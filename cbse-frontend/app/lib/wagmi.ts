'use client';

import { getDefaultConfig } from '@rainbow-me/rainbowkit';
import { http } from 'wagmi';
import { mainnet, sepolia, bscTestnet } from 'wagmi/chains';

// Configure chains - EVM Compatible
export const config = getDefaultConfig({
  appName: 'VeriPro - AI Security Scanner',
  projectId: process.env.NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID || '00000000000000000000000000000000',
  chains: [mainnet, sepolia, bscTestnet],
  transports: {
    [mainnet.id]: http(),
    [sepolia.id]: http(),
    [bscTestnet.id]: http(),
  },
  ssr: true,
});
