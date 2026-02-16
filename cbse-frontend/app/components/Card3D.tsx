'use client';

import { motion } from 'framer-motion';
import { ReactNode } from 'react';

interface Card3DProps {
    children: ReactNode;
    delay?: number;
}

export default function Card3D({ children, delay = 0 }: Card3DProps) {
    return (
        <motion.div
            initial={{ opacity: 0, y: 30 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6, delay }}
            whileHover={{
                scale: 1.02,
                rotateX: 2,
                rotateY: 2,
                z: 50,
                transition: { duration: 0.3 }
            }}
            className="group relative border border-zinc-800 bg-black p-8 cursor-pointer"
            style={{
                transformStyle: 'preserve-3d',
                perspective: '1000px'
            }}
        >
            {/* 3D Shadow effect */}
            <div className="absolute inset-0 bg-gradient-to-br from-zinc-900/50 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300" />

            {/* Bottom shadow */}
            <div className="absolute -bottom-1 -right-1 w-full h-full bg-zinc-900/50 -z-10 group-hover:translate-x-1 group-hover:translate-y-1 transition-transform duration-300" />

            <div className="relative z-10">
                {children}
            </div>
        </motion.div>
    );
}
