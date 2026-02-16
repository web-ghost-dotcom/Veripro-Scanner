import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Webpack configuration to handle problematic packages
  webpack: (config, { isServer }) => {
    // Handle pino and related packages for client-side
    if (!isServer) {
      config.resolve.fallback = {
        ...config.resolve.fallback,
        fs: false,
        net: false,
        tls: false,
        child_process: false,
        worker_threads: false,
      };
    }

    // Ignore problematic imports
    config.resolve.alias = {
      ...config.resolve.alias,
      'pino-pretty': false,
    };

    return config;
  },

  // Transpile packages that need it
  transpilePackages: [
    '@walletconnect/universal-provider',
    '@walletconnect/ethereum-provider',
  ],
};

export default nextConfig;
