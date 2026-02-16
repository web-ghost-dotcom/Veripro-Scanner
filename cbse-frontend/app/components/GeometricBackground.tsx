'use client';

import { useRef, useMemo } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { Sphere, Box, Torus } from '@react-three/drei';
import * as THREE from 'three';

function FloatingGeometry({ position, geometry }: { position: [number, number, number]; geometry: 'sphere' | 'box' | 'torus' }) {
    const meshRef = useRef<THREE.Mesh>(null);

    useFrame((state) => {
        if (!meshRef.current) return;
        meshRef.current.rotation.x = state.clock.getElapsedTime() * 0.2;
        meshRef.current.rotation.y = state.clock.getElapsedTime() * 0.3;
        meshRef.current.position.y = position[1] + Math.sin(state.clock.getElapsedTime() * 0.5) * 0.3;
    });

    const GeometryComponent = geometry === 'sphere' ? Sphere : geometry === 'box' ? Box : Torus;

    return (
        <GeometryComponent ref={meshRef} position={position} args={geometry === 'torus' ? [0.5, 0.2, 16, 100] : [0.8, 0.8, 0.8]}>
            <meshStandardMaterial color="#ffffff" wireframe opacity={0.15} transparent />
        </GeometryComponent>
    );
}

function NetworkNodes() {
    const groupRef = useRef<THREE.Group>(null);

    // Generate positions once with useMemo to avoid impure function during render
    const nodePositions = useMemo(() => {
        /* eslint-disable react-hooks/purity */
        return Array.from({ length: 20 }).map((_, i) => {
            const angle = (i / 20) * Math.PI * 2;
            const radius = 4;
            const x = Math.cos(angle) * radius;
            const z = Math.sin(angle) * radius;
            const y = (Math.random() - 0.5) * 3;
            return { x, y, z };
        });
        /* eslint-enable react-hooks/purity */
    }, []);

    useFrame((state) => {
        if (!groupRef.current) return;
        groupRef.current.rotation.y = state.clock.getElapsedTime() * 0.05;
    });

    return (
        <group ref={groupRef}>
            {nodePositions.map((pos, i) => (
                <Sphere key={i} position={[pos.x, pos.y, pos.z]} args={[0.05, 8, 8]}>
                    <meshStandardMaterial color="#ffffff" emissive="#ffffff" emissiveIntensity={0.3} />
                </Sphere>
            ))}
        </group>
    );
}

export default function GeometricBackground() {
    return (
        <div className="absolute inset-0 opacity-20 pointer-events-none">
            <Canvas camera={{ position: [0, 0, 8], fov: 50 }} style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%' }}>
                <ambientLight intensity={0.5} />
                <pointLight position={[10, 10, 10]} intensity={1} />
                <FloatingGeometry position={[-2, 0, 0]} geometry="box" />
                <FloatingGeometry position={[2, 1, -2]} geometry="sphere" />
                <FloatingGeometry position={[0, -1, -1]} geometry="torus" />
                <NetworkNodes />
            </Canvas>
        </div>
    );
}
