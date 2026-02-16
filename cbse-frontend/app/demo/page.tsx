'use client';

import dynamic from 'next/dynamic';
import { motion } from 'framer-motion';
import DemoInterface from '../components/DemoInterface';

const GeometricBackground = dynamic(() => import('../components/GeometricBackground'), { ssr: false });

export default function DemoPage() {
    return (
        <div className="min-h-screen bg-black text-white pt-20">
            <section className="relative min-h-[calc(100vh-80px)] overflow-hidden">
                <GeometricBackground />

                <div className="relative z-10 max-w-7xl mx-auto px-6 py-12">
                    <motion.div
                        initial={{ opacity: 0, y: 20 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ duration: 0.8 }}
                        className="text-center mb-12"
                    >
                        <h1 className="text-5xl font-light tracking-tight mb-4">
                            Protocol Demo
                        </h1>
                        <p className="text-zinc-400 max-w-2xl mx-auto">
                            Submit example smart contracts to verify the VeriPro technology.
                            This is a demonstration environment.
                        </p>
                    </motion.div>

                    <DemoInterface />
                </div>
            </section>
        </div>
    );
}
