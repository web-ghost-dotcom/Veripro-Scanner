'use client';

import dynamic from 'next/dynamic';
import { motion } from 'framer-motion';
import { useState } from 'react';
import { useRouter } from 'next/navigation';
import CodeVisualization from './components/CodeVisualization';
import SpecificationExample from './components/SpecificationExample';
import VerificationOutput from './components/VerificationOutput';
import StreamingTerminal from './components/StreamingTerminal';
import Card3D from './components/Card3D';
import DemoInterface from './components/DemoInterface';

const GeometricBackground = dynamic(() => import('./components/GeometricBackground'), { ssr: false });

export default function Home() {
  const router = useRouter();
  const [email, setEmail] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitStatus, setSubmitStatus] = useState<'idle' | 'success' | 'error'>('idle');

  const handleWaitlistSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      setSubmitStatus('error');
      return;
    }
    setIsSubmitting(true);
    await new Promise(resolve => setTimeout(resolve, 1000));
    setSubmitStatus('success');
    setIsSubmitting(false);
    setEmail('');
    setTimeout(() => setSubmitStatus('idle'), 3000);
  };

  const terminalLines = [
    '$ veripro verify TokenContract.sol',
    'Loading contract bytecode...',
    'Building symbolic state...',
    'Exploring execution paths...',
    '  [OK] Path 1/256: SAT (42ms)',
    '  [OK] Path 128/256: SAT (38ms)',
    '  [OK] Path 256/256: SAT (45ms)',
    'All assertions verified successfully.',
    'Total time: 1.2s'
  ];

  return (
    <div className="min-h-screen bg-black text-white pt-20">
      {/* Hero Section */}
      <section className="relative min-h-[calc(100vh-80px)] flex items-center justify-center overflow-hidden">
        <GeometricBackground />

        <div className="relative z-10 max-w-7xl mx-auto px-6 py-24 text-center">
          <motion.div
            initial={{ opacity: 0, y: 30 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 1, ease: "easeOut" }}
          >
            <h1 className="text-7xl md:text-8xl font-light tracking-tight mb-6 bg-clip-text text-transparent bg-gradient-to-r from-blue-400 to-blue-600">
              VERIPRO
            </h1>
            <p className="text-xl md:text-2xl font-light text-zinc-400 mb-4 tracking-wide">
              AI Security Scanner & Formal Verification Platform
            </p>
            <p className="text-base md:text-lg text-zinc-500 max-w-2xl mx-auto mb-12 leading-relaxed">
              Secure your smart contracts with our AI Agent.
              Scan for vulnerabilities, write formal specifications, mathematically prove correctness, and publish your verification results on-chain.
            </p>

            <div className="flex gap-6 justify-center items-center flex-wrap">
              <motion.button
                onClick={() => router.push('/app')}
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
                className="px-8 py-4 bg-white text-black font-medium tracking-wide hover:bg-zinc-200 transition-colors cursor-pointer"
              >
                Open Platform
              </motion.button>
              <motion.a
                href="/docs"
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
                className="px-8 py-4 border border-zinc-700 font-medium tracking-wide hover:border-zinc-500 transition-colors inline-block"
              >
                Documentation
              </motion.a>
            </div>
          </motion.div>
        </div>
      </section>

      {/* What is Veripro Section */}
      <section className="relative bg-black py-32 border-t border-zinc-900">
        <div className="relative z-10 max-w-7xl mx-auto px-6">
          <motion.div
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: false, amount: 0.3 }}
            transition={{ duration: 0.6 }}
            className="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center"
          >
            <div>
              <h2 className="text-4xl md:text-5xl font-light mb-8 tracking-tight">
                AI-Powered Contract Security
              </h2>
              <div className="space-y-6 text-zinc-400 leading-relaxed">
                <p>
                  VeriPro is an <span className="text-blue-400">AI Security Agent</span> that proactively scans your smart contracts. <span className="text-white">Import from GitHub, upload files, or paste code directly</span>.
                </p>
                <p>
                  Our agent identifies potential vulnerabilities (like Reentrancy or ERC-20 issues), writes formal specifications, and runs the <span className="text-blue-400">CBSE symbolic engine</span> to mathematically prove safety.
                </p>
              </div>

              <div className="mt-8 grid grid-cols-2 gap-8">
                <div>
                  <div className="text-2xl font-light mb-2 text-white">AI Agent</div>
                  <div className="text-sm text-zinc-500">Autonomous vulnerability scanning & spec generation.</div>
                </div>
                <div>
                  <div className="text-2xl font-light mb-2 text-white">Formal Proof</div>
                  <div className="text-sm text-zinc-500">Zero false-positive verification with mathematical proofs.</div>
                </div>
                <div>
                  <div className="text-2xl font-light mb-2 text-white">Write Specs</div>
                  <div className="text-sm text-zinc-500">Full control to write custom formal specifications.</div>
                </div>
                <div>
                  <div className="text-2xl font-light mb-2 text-white">Publish</div>
                  <div className="text-sm text-zinc-500">Commit verification results on-chain for public trust.</div>
                </div>
              </div>
            </div>
            <div>
              <StreamingTerminal lines={terminalLines} speed={40} />
            </div>
          </motion.div>
        </div>
      </section>

      {/* Demo Video Section */}
      <section className="relative bg-black py-32 border-t border-zinc-900">
        <GeometricBackground />

        <div className="relative z-10 max-w-6xl mx-auto px-6">
          <motion.div
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: false, amount: 0.3 }}
            transition={{ duration: 0.8 }}
          >
            <h2 className="text-4xl md:text-5xl font-light text-center mb-4 tracking-tight">
              See it in action
            </h2>
            <p className="text-center text-zinc-500 mb-16 text-lg">
              Watch how VeriPro verifies complex smart contract invariants
            </p>

            <div className="aspect-video bg-zinc-900 border border-zinc-800 flex items-center justify-center relative overflow-hidden group">
              <div className="absolute inset-0 bg-gradient-to-br from-zinc-900 via-zinc-900 to-black opacity-50" />
              <motion.div
                whileHover={{ scale: 1.1 }}
                className="relative z-10 w-20 h-20 border-2 border-white rounded-full flex items-center justify-center cursor-pointer"
              >
                <div className="w-0 h-0 border-t-8 border-t-transparent border-l-12 border-l-white border-b-8 border-b-transparent ml-1" />
              </motion.div>
              <div className="absolute bottom-6 left-6 text-zinc-600 text-sm font-mono">
                Watch Demo
              </div>
            </div>
          </motion.div>
        </div>
      </section>

      {/* Demo Interface Section */}
      <section className="relative bg-black py-24 border-t border-zinc-900">
        <div className="relative z-10 max-w-7xl mx-auto px-6 mb-12 text-center">
          <h2 className="text-4xl md:text-5xl font-light mb-6 tracking-tight">
            Try the Demo
          </h2>
          <p className="text-xl text-zinc-400 max-w-2xl mx-auto leading-relaxed">
            Verify this example contract instantly.
          </p>
        </div>
        <DemoInterface />
      </section>

      {/* Features Section */}
      <section className="relative bg-black py-32 border-t border-zinc-900">
        <GeometricBackground />

        <div className="relative z-10 max-w-7xl mx-auto px-6">
          <motion.h2
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: false, amount: 0.3 }}
            className="text-4xl md:text-5xl font-light mb-20 tracking-tight text-center"
          >
            Technical Capabilities
          </motion.h2>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
            {[
              {
                title: 'Project Workspace',
                description: 'Import contracts from GitHub repositories, upload project folders, or paste code directly. Manage multiple projects with full file tree navigation and version control integration.'
              },
              {
                title: 'Specification Editor',
                description: 'Write formal specifications alongside your contracts. Define invariants, pre-conditions, and security properties that VeriPro will mathematically verify across all execution paths.'
              },
              {
                title: 'Symbolic Execution Engine',
                description: 'State-of-the-art symbolic executor explores all paths simultaneously. Leverages Z3 SMT solver for constraint solving with full EVM opcode support including DELEGATECALL and CREATE2.'
              },
              {
                title: 'On-Chain Attestation',
                description: 'Successful verifications generate cryptographic attestations. Publish proof on-chain that your contract meets its specification, providing verifiable security guarantees to users.'
              }
            ].map((feature, i) => (
              <Card3D key={i} delay={i * 0.1}>
                <h3 className="text-xl font-medium mb-3 tracking-wide">{feature.title}</h3>
                <p className="text-zinc-500 leading-relaxed">{feature.description}</p>
              </Card3D>
            ))}
          </div>
        </div>
      </section>

      {/* Specification Example Section */}
      <section className="relative bg-black py-32 border-t border-zinc-900">
        <GeometricBackground />

        <div className="relative z-10 max-w-6xl mx-auto px-6">
          <motion.div
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: false, amount: 0.3 }}
            transition={{ duration: 0.8 }}
          >
            <h2 className="text-4xl md:text-5xl font-light text-center mb-4 tracking-tight">
              Write specifications, not tests
            </h2>
            <p className="text-center text-zinc-500 mb-16 text-lg max-w-3xl mx-auto">
              Express your security properties as formal assertions. VeriPro automatically generates
              test cases covering all possible behaviors, finding bugs that manual testing would miss.
            </p>
            <SpecificationExample />
          </motion.div>
        </div>
      </section>

      {/* Verification Output Wall Section */}
      <section className="relative bg-black py-32 border-t border-zinc-900">
        <GeometricBackground />

        <div className="relative z-10 max-w-7xl mx-auto px-6">
          <motion.div
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: false, amount: 0.3 }}
            transition={{ duration: 0.8 }}
          >
            <h2 className="text-4xl md:text-5xl font-light text-center mb-4 tracking-tight">
              Verification in action
            </h2>
            <p className="text-center text-zinc-500 mb-16 text-lg">
              Real-time feedback with detailed counterexamples and proof traces
            </p>
            <VerificationOutput />
          </motion.div>
        </div>
      </section>

      {/* Code Example Section */}
      <section className="relative bg-black py-32 border-t border-zinc-900">
        <GeometricBackground />

        <div className="relative z-10 max-w-4xl mx-auto px-6">
          <motion.div
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: false, amount: 0.3 }}
            transition={{ duration: 0.8 }}
          >
            <h2 className="text-3xl md:text-4xl font-light mb-6 tracking-tight">
              Verification for developers
            </h2>
            <p className="text-zinc-500 mb-12 text-lg">
              Write specifications in familiar Solidity syntax. VeriPro handles the complexity.
            </p>
            <CodeVisualization />
          </motion.div>
        </div>
      </section>

      {/* CTA Section */}
      <section id="waitlist" className="relative bg-black py-32 border-t border-zinc-900">
        <GeometricBackground />

        <div className="relative z-10 max-w-4xl mx-auto px-6 text-center">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: false, amount: 0.3 }}
            transition={{ duration: 0.8 }}
          >
            <h2 className="text-4xl md:text-5xl font-light mb-6 tracking-tight">
              Get started today
            </h2>
            <p className="text-zinc-500 mb-12 text-lg max-w-2xl mx-auto">
              Join the waitlist for priority access. Be among the first teams to verify their
              smart contracts with cryptographic proof.
            </p>

            <form onSubmit={handleWaitlistSubmit} className="max-w-md mx-auto">
              <div className="flex flex-col sm:flex-row gap-3">
                <input
                  type="email"
                  value={email}
                  onChange={(e) => {
                    setEmail(e.target.value);
                    setSubmitStatus('idle');
                  }}
                  placeholder="Enter your email"
                  className="flex-1 px-6 py-4 bg-zinc-900 border border-zinc-800 text-white placeholder-zinc-600 focus:outline-none focus:border-zinc-600 transition-colors"
                  disabled={isSubmitting}
                />
                <motion.button
                  type="submit"
                  whileHover={{ scale: isSubmitting ? 1 : 1.05 }}
                  whileTap={{ scale: isSubmitting ? 1 : 0.95 }}
                  disabled={isSubmitting}
                  className={`px-8 py-4 font-medium tracking-wide transition-colors ${isSubmitting
                    ? 'bg-zinc-800 text-zinc-600 cursor-not-allowed'
                    : 'bg-white text-black hover:bg-zinc-200'
                    }`}
                >
                  {isSubmitting ? 'Joining...' : 'Join Waitlist'}
                </motion.button>
              </div>

              {submitStatus === 'success' && (
                <motion.div
                  initial={{ opacity: 0, y: -10 }}
                  animate={{ opacity: 1, y: 0 }}
                  className="mt-4 text-green-400 text-sm"
                >
                  You have been added to the list. We will be in touch soon.
                </motion.div>
              )}

              {submitStatus === 'error' && (
                <motion.div
                  initial={{ opacity: 0, y: -10 }}
                  animate={{ opacity: 1, y: 0 }}
                  className="mt-4 text-red-400 text-sm"
                >
                  Please enter a valid email address.
                </motion.div>
              )}
            </form>
          </motion.div>
        </div>
      </section>

      {/* Footer */}
      <footer className="relative border-t border-zinc-900 bg-black py-12">
        <div className="relative z-10 max-w-7xl mx-auto px-6">
          <div className="flex flex-col md:flex-row justify-between items-center gap-6">
            <div className="text-zinc-600 text-sm">
              © 2025 VeriPro. Complete Smart Contract Verification Platform.
            </div>
            <div className="flex gap-8 text-sm">
              <a href="/docs" className="text-zinc-500 hover:text-white transition-colors">Documentation</a>
              <a href="#" className="text-zinc-500 hover:text-white transition-colors">GitHub</a>
              <a href="#" className="text-zinc-500 hover:text-white transition-colors">Research</a>
              <a href="#" className="text-zinc-500 hover:text-white transition-colors">Contact</a>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
}
