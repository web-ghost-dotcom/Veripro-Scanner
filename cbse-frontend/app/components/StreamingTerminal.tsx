'use client';

import { useState, useEffect, useRef } from 'react';
import { motion } from 'framer-motion';

interface StreamingTerminalProps {
    lines: string[];
    speed?: number;
}

export default function StreamingTerminal({ lines, speed = 30 }: StreamingTerminalProps) {
    const [displayedText, setDisplayedText] = useState('');
    const [currentLineIndex, setCurrentLineIndex] = useState(0);
    const [charIndex, setCharIndex] = useState(0);
    const [isInView, setIsInView] = useState(false);
    const ref = useRef<HTMLDivElement>(null);

    // Intersection Observer to detect when component enters viewport
    useEffect(() => {
        const element = ref.current;
        const observer = new IntersectionObserver(
            ([entry]) => {
                if (entry.isIntersecting) {
                    setIsInView(true);
                    // Reset animation when entering viewport
                    setDisplayedText('');
                    setCurrentLineIndex(0);
                    setCharIndex(0);
                } else {
                    setIsInView(false);
                }
            },
            { threshold: 0.3 }
        );

        if (element) {
            observer.observe(element);
        }

        return () => {
            if (element) {
                observer.unobserve(element);
            }
        };
    }, []);

    useEffect(() => {
        if (!isInView || currentLineIndex >= lines.length) return;

        const currentLine = lines[currentLineIndex];

        if (charIndex < currentLine.length) {
            const timer = setTimeout(() => {
                setDisplayedText(prev => prev + currentLine[charIndex]);
                setCharIndex(charIndex + 1);
            }, speed);
            return () => clearTimeout(timer);
        } else if (currentLineIndex < lines.length - 1) {
            const timer = setTimeout(() => {
                setDisplayedText(prev => prev + '\n');
                setCurrentLineIndex(currentLineIndex + 1);
                setCharIndex(0);
            }, speed * 3);
            return () => clearTimeout(timer);
        }
    }, [charIndex, currentLineIndex, lines, speed, isInView]);

    return (
        <motion.div
            ref={ref}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="relative rounded-sm border border-zinc-800 bg-black p-6 font-mono text-xs md:text-sm overflow-hidden"
        >
            <div className="absolute top-3 left-3 flex gap-1.5">
                <div className="h-2 w-2 rounded-full bg-zinc-700" />
                <div className="h-2 w-2 rounded-full bg-zinc-700" />
                <div className="h-2 w-2 rounded-full bg-zinc-700" />
            </div>
            <pre className="mt-6 text-zinc-400 whitespace-pre-wrap overflow-x-auto">
                {displayedText}
                {isInView && <span className="inline-block w-2 h-4 bg-zinc-500 ml-1 animate-pulse" />}
            </pre>
        </motion.div>
    );
}
