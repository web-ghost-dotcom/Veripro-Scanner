'use client';

import { useRef, useMemo } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import * as THREE from 'three';

function ParticleField() {
    const pointsRef = useRef<THREE.Points>(null);
    const particlesCount = 1000;

    const positions = useMemo(() => {
        /* eslint-disable react-hooks/purity */
        const pos = new Float32Array(particlesCount * 3);
        for (let i = 0; i < particlesCount; i++) {
            pos[i * 3] = (Math.random() - 0.5) * 20;
            pos[i * 3 + 1] = (Math.random() - 0.5) * 20;
            pos[i * 3 + 2] = (Math.random() - 0.5) * 20;
        }
        return pos;
        /* eslint-enable react-hooks/purity */
    }, []);

    useFrame((state) => {
        if (!pointsRef.current) return;
        pointsRef.current.rotation.y = state.clock.getElapsedTime() * 0.05;
    });

    return (
        <points ref={pointsRef}>
            <bufferGeometry>
                <bufferAttribute
                    attach="attributes-position"
                    count={particlesCount}
                    array={positions}
                    itemSize={3}
                    args={[positions, 3]}
                />
            </bufferGeometry>
            <pointsMaterial size={0.03} color="#ffffff" transparent opacity={0.6} sizeAttenuation />
        </points>
    );
}

export default function ParticleBackground() {
    return (
        <div className="absolute inset-0 opacity-20">
            <Canvas camera={{ position: [0, 0, 5], fov: 75 }}>
                <ParticleField />
            </Canvas>
        </div>
    );
}
