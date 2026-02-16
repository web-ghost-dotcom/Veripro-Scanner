'use client';

import { motion, AnimatePresence } from 'framer-motion';
import { useState, useEffect } from 'react';

export default function LightningFlash() {
    const [flashes, setFlashes] = useState<{ id: number; x: number; y: number }[]>([]);
    const [nextId, setNextId] = useState(0);

    useEffect(() => {
        const createFlash = () => {
            // Random position
            const x = Math.random() * 100;
            const y = Math.random() * 100;

            const flash = { id: nextId, x, y };
            setFlashes(prev => [...prev, flash]);
            setNextId(prev => prev + 1);

            // Remove flash after animation
            setTimeout(() => {
                setFlashes(prev => prev.filter(f => f.id !== flash.id));
            }, 2000);
        };

        // Random interval between 8-15 seconds for more subtle effect
        const scheduleNext = () => {
            const delay = 8000 + Math.random() * 7000;
            setTimeout(() => {
                createFlash();
                scheduleNext();
            }, delay);
        };

        scheduleNext();
    }, [nextId]);

    return (
        <div className="fixed inset-0 pointer-events-none z-[5]">
            <AnimatePresence>
                {flashes.map((flash) => (
                    <motion.div
                        key={flash.id}
                        initial={{ opacity: 0, scale: 0 }}
                        animate={{
                            opacity: [0, 0.15, 0.25, 0.15, 0],
                            scale: [0.5, 1, 1.1, 1, 0.8]
                        }}
                        exit={{ opacity: 0 }}
                        transition={{ duration: 1.5, times: [0, 0.2, 0.4, 0.6, 1] }}
                        style={{
                            position: 'absolute',
                            left: `${flash.x}%`,
                            top: `${flash.y}%`,
                        }}
                        className="text-white"
                    >
                        <svg
                            width="24"
                            height="24"
                            viewBox="0 0 24 24"
                            fill="currentColor"
                            className="opacity-40"
                        >
                            <path d="M13 2L3 14h8l-1 8 10-12h-8l1-8z" />
                        </svg>
                    </motion.div>
                ))}
            </AnimatePresence>
        </div>
    );
}
