'use client';

import { motion, AnimatePresence } from 'framer-motion';
import { useState, useEffect, useRef } from 'react';
import Link from 'next/link';

// ===== TYPES =====
interface TutorialStep {
    id: string;
    title: string;
    description: string;
    code?: string;
    highlight?: string;
}

interface FlowNode {
    id: string;
    label: string;
    x: number;
    y: number;
}

// ===== NAVIGATION =====
const sections = [
    { id: 'overview', title: 'Overview' },
    { id: 'ai-security', title: 'AI Security Agent' },
    { id: 'how-it-works', title: 'How It Works' },
    { id: 'architecture', title: 'Architecture' },
    { id: 'tutorial', title: 'Interactive Tutorial' },
    { id: 'writing-specs', title: 'Writing Specifications' },
    { id: 'api-reference', title: 'API Reference' },
    { id: 'examples', title: 'Examples' },
];

// ===== ANIMATED FLOW DIAGRAM COMPONENT =====
function AnimatedFlowDiagram() {
    const [activeNode, setActiveNode] = useState<string | null>(null);
    const [animationStep, setAnimationStep] = useState(0);

    useEffect(() => {
        const interval = setInterval(() => {
            setAnimationStep((prev) => (prev + 1) % 6);
        }, 2000);
        return () => clearInterval(interval);
    }, []);

    const nodes: FlowNode[] = [
        { id: 'contract', label: 'Smart Contract', x: 50, y: 50 },
        { id: 'spec', label: 'Specification', x: 200, y: 50 },
        { id: 'compile', label: 'Compile', x: 125, y: 130 },
        { id: 'symbolic', label: 'Symbolic Execution', x: 125, y: 210 },
        { id: 'solver', label: 'SMT Solver', x: 125, y: 290 },
        { id: 'result', label: 'Verification Result', x: 125, y: 370 },
    ];

    const edges = [
        { from: 'contract', to: 'compile' },
        { from: 'spec', to: 'compile' },
        { from: 'compile', to: 'symbolic' },
        { from: 'symbolic', to: 'solver' },
        { from: 'solver', to: 'result' },
    ];

    const getNodePosition = (id: string) => {
        const node = nodes.find(n => n.id === id);
        return node ? { x: node.x + 60, y: node.y + 20 } : { x: 0, y: 0 };
    };

    const isEdgeActive = (toId: string) => {
        const edgeMap: Record<number, string[]> = {
            0: ['contract', 'spec'],
            1: ['compile'],
            2: ['symbolic'],
            3: ['solver'],
            4: ['result'],
            5: [],
        };
        const activeNodes = edgeMap[animationStep] || [];
        return activeNodes.includes(toId);
    };

    return (
        <div className="relative w-full h-[450px] bg-zinc-950 rounded-sm border border-zinc-800 overflow-hidden">
            {/* Background grid */}
            <div className="absolute inset-0 opacity-5">
                <svg width="100%" height="100%">
                    <defs>
                        <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
                            <path d="M 40 0 L 0 0 0 40" fill="none" stroke="white" strokeWidth="0.5" />
                        </pattern>
                    </defs>
                    <rect width="100%" height="100%" fill="url(#grid)" />
                </svg>
            </div>

            <svg width="100%" height="100%" className="relative z-10">
                {/* Edges */}
                {edges.map((edge, i) => {
                    const from = getNodePosition(edge.from);
                    const to = getNodePosition(edge.to);
                    const isActive = isEdgeActive(edge.to);

                    return (
                        <g key={i}>
                            <line
                                x1={from.x}
                                y1={from.y}
                                x2={to.x}
                                y2={to.y}
                                stroke={isActive ? '#fff' : '#3f3f46'}
                                strokeWidth={isActive ? 2 : 1}
                                strokeDasharray={isActive ? '0' : '4,4'}
                                className="transition-all duration-500"
                            />
                            {isActive && (
                                <motion.circle
                                    r="3"
                                    fill="#fff"
                                    initial={{ cx: from.x, cy: from.y }}
                                    animate={{ cx: to.x, cy: to.y }}
                                    transition={{ duration: 1, ease: 'easeInOut' }}
                                />
                            )}
                        </g>
                    );
                })}

                {/* Nodes */}
                {nodes.map((node) => {
                    const isActive = animationStep >= nodes.findIndex(n => n.id === node.id);

                    return (
                        <g
                            key={node.id}
                            onMouseEnter={() => setActiveNode(node.id)}
                            onMouseLeave={() => setActiveNode(null)}
                            className="cursor-pointer"
                        >
                            <motion.rect
                                x={node.x}
                                y={node.y}
                                width="120"
                                height="40"
                                rx="2"
                                fill={isActive ? '#18181b' : '#0a0a0a'}
                                stroke={activeNode === node.id ? '#fff' : isActive ? '#52525b' : '#27272a'}
                                strokeWidth={activeNode === node.id ? 2 : 1}
                                initial={{ opacity: 0, scale: 0.8 }}
                                animate={{ opacity: 1, scale: 1 }}
                                transition={{ delay: nodes.findIndex(n => n.id === node.id) * 0.1 }}
                                className="transition-all duration-300"
                            />
                            <text
                                x={node.x + 60}
                                y={node.y + 25}
                                textAnchor="middle"
                                fill={isActive ? '#fff' : '#71717a'}
                                fontSize="11"
                                fontWeight="400"
                                className="pointer-events-none transition-colors"
                            >
                                {node.label}
                            </text>
                        </g>
                    );
                })}
            </svg>

            {/* Legend */}
            <div className="absolute bottom-4 left-4 flex gap-6 text-xs text-zinc-600">
                <span className="flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-zinc-500" /> Input
                </span>
                <span className="flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-zinc-400" /> Processing
                </span>
                <span className="flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-white" /> Output
                </span>
            </div>

            {/* Step indicator */}
            <div className="absolute bottom-4 right-4 flex gap-1">
                {[0, 1, 2, 3, 4, 5].map((step) => (
                    <div
                        key={step}
                        className={`w-1.5 h-1.5 rounded-full transition-colors ${animationStep === step ? 'bg-white' : 'bg-zinc-800'
                            }`}
                    />
                ))}
            </div>
        </div>
    );
}

// ===== ARCHITECTURE DIAGRAM =====
function ArchitectureDiagram() {
    const [hoveredComponent, setHoveredComponent] = useState<string | null>(null);

    const components = [
        {
            id: 'frontend',
            name: 'VeriPro Frontend',
            desc: 'React/Next.js interface for project management and verification',
            x: 20,
            y: 20,
            width: 200,
            height: 80,
            layer: 'UI Layer'
        },
        {
            id: 'api',
            name: 'API Gateway',
            desc: 'Next.js API routes handling verification requests',
            x: 240,
            y: 20,
            width: 160,
            height: 80,
            layer: 'UI Layer'
        },
        {
            id: 'coordinator',
            name: 'CBSE Coordinator',
            desc: 'Rust service orchestrating verification jobs',
            x: 130,
            y: 130,
            width: 180,
            height: 80,
            layer: 'Service Layer'
        },
        {
            id: 'forge',
            name: 'Forge Compiler',
            desc: 'Compiles Solidity to bytecode and generates ABI',
            x: 20,
            y: 240,
            width: 140,
            height: 70,
            layer: 'Execution Layer'
        },
        {
            id: 'cbse',
            name: 'CBSE Engine',
            desc: 'Symbolic EVM executing all paths with Z3 solver',
            x: 180,
            y: 240,
            width: 140,
            height: 70,
            layer: 'Execution Layer'
        },
        {
            id: 'protocol',
            name: 'Attestation Protocol',
            desc: 'Signs verification results for on-chain commitment',
            x: 340,
            y: 240,
            width: 140,
            height: 70,
            layer: 'Execution Layer'
        },
        {
            id: 'blockchain',
            name: 'Blockchain',
            desc: 'On-chain registry storing verification attestations',
            x: 180,
            y: 340,
            width: 140,
            height: 60,
            layer: 'Blockchain Layer'
        },
    ];

    const connections = [
        { from: 'frontend', to: 'api' },
        { from: 'api', to: 'coordinator' },
        { from: 'coordinator', to: 'forge' },
        { from: 'coordinator', to: 'cbse' },
        { from: 'cbse', to: 'protocol' },
        { from: 'protocol', to: 'blockchain' },
    ];

    return (
        <div className="relative w-full bg-zinc-950 rounded-sm border border-zinc-800 p-6">
            <div className="flex justify-between items-start mb-6">
                <div>
                    <h3 className="text-lg font-light text-white">System Architecture</h3>
                    <p className="text-sm text-zinc-600">Hover over components to learn more</p>
                </div>
                <div className="flex gap-6 text-xs text-zinc-600">
                    {['UI', 'Service', 'Execution', 'Blockchain'].map((layer) => (
                        <span key={layer} className="flex items-center gap-1.5">
                            <span className="w-1.5 h-1.5 rounded-full bg-zinc-600" />
                            {layer}
                        </span>
                    ))}
                </div>
            </div>

            <svg width="100%" height="420" viewBox="0 0 500 420">
                {/* Connection lines */}
                {connections.map((conn, i) => {
                    const from = components.find(c => c.id === conn.from)!;
                    const to = components.find(c => c.id === conn.to)!;
                    const fromX = from.x + from.width / 2;
                    const fromY = from.y + from.height;
                    const toX = to.x + to.width / 2;
                    const toY = to.y;

                    return (
                        <path
                            key={i}
                            d={`M ${fromX} ${fromY} C ${fromX} ${fromY + 30}, ${toX} ${toY - 30}, ${toX} ${toY}`}
                            fill="none"
                            stroke={hoveredComponent === conn.from || hoveredComponent === conn.to ? '#fff' : '#27272a'}
                            strokeWidth={hoveredComponent === conn.from || hoveredComponent === conn.to ? 2 : 1}
                            strokeDasharray={hoveredComponent ? '0' : '4,4'}
                            className="transition-all duration-300"
                            markerEnd="url(#arrowhead)"
                        />
                    );
                })}

                {/* Arrowhead marker */}
                <defs>
                    <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
                        <polygon points="0 0, 10 3.5, 0 7" fill="#27272a" />
                    </marker>
                </defs>

                {/* Component boxes */}
                {components.map((comp) => (
                    <g
                        key={comp.id}
                        onMouseEnter={() => setHoveredComponent(comp.id)}
                        onMouseLeave={() => setHoveredComponent(null)}
                        className="cursor-pointer"
                    >
                        <motion.rect
                            x={comp.x}
                            y={comp.y}
                            width={comp.width}
                            height={comp.height}
                            rx="2"
                            fill={hoveredComponent === comp.id ? '#18181b' : '#0a0a0a'}
                            stroke={hoveredComponent === comp.id ? '#fff' : '#27272a'}
                            strokeWidth={hoveredComponent === comp.id ? 2 : 1}
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            transition={{ delay: components.indexOf(comp) * 0.1 }}
                        />
                        <text
                            x={comp.x + comp.width / 2}
                            y={comp.y + comp.height / 2}
                            textAnchor="middle"
                            dominantBaseline="middle"
                            fill={hoveredComponent === comp.id ? '#fff' : '#71717a'}
                            fontSize="12"
                            fontWeight="400"
                            className="transition-colors"
                        >
                            {comp.name}
                        </text>
                    </g>
                ))}
            </svg>

            {/* Component details panel */}
            <AnimatePresence>
                {hoveredComponent && (
                    <motion.div
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, y: 10 }}
                        className="absolute bottom-6 left-6 right-6 p-4 bg-zinc-900 border border-zinc-800 rounded-sm"
                    >
                        {(() => {
                            const comp = components.find(c => c.id === hoveredComponent);
                            return comp ? (
                                <div>
                                    <h4 className="font-medium text-white">{comp.name}</h4>
                                    <p className="text-sm text-zinc-400 mt-1">{comp.desc}</p>
                                    <span className="text-xs text-zinc-600 mt-2 inline-block">{comp.layer}</span>
                                </div>
                            ) : null;
                        })()}
                    </motion.div>
                )}
            </AnimatePresence>
        </div>
    );
}

// ===== INTERACTIVE TUTORIAL =====
function InteractiveTutorial() {
    const [currentStep, setCurrentStep] = useState(0);
    const [isPlaying, setIsPlaying] = useState(false);

    const steps: TutorialStep[] = [
        {
            id: 'create-project',
            title: '1. Create a New Project',
            description: 'Start by creating a new project in VeriPro. Upload your Solidity contract or paste the code directly.',
            code: `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

contract SimpleVault {
    mapping(address => uint256) public balances;
    
    function deposit() external payable {
        balances[msg.sender] += msg.value;
    }
    
    function withdraw(uint256 amount) external {
        require(balances[msg.sender] >= amount, "Insufficient balance");
        balances[msg.sender] -= amount;
        payable(msg.sender).transfer(amount);
    }
}`,
            highlight: 'contract'
        },
        {
            id: 'write-spec',
            title: '2. Write Formal Specifications',
            description: 'Define properties that must always hold. VeriPro will prove these for ALL possible inputs.',
            code: `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "./SimpleVault.sol";

contract VaultTest is Test {
    SimpleVault vault;
    
    function setUp() public {
        vault = new SimpleVault();
    }
    
    // Property: Balance never becomes negative
    function test_balanceNeverNegative(address user) public view {
        assertGe(vault.balances(user), 0);
    }
    
    // Property: Deposit increases balance correctly
    function test_depositIncreasesBalance(uint256 amount) public {
        vm.assume(amount > 0 && amount < 100 ether);
        
        uint256 before = vault.balances(address(this));
        vault.deposit{value: amount}();
        uint256 afterBal = vault.balances(address(this));
        
        assertEq(afterBal, before + amount);
    }
}`,
            highlight: 'spec'
        },
        {
            id: 'run-verify',
            title: '3. Run Verification',
            description: 'Click "Run Verification" to start the symbolic execution. VeriPro explores ALL possible execution paths.',
            code: `+-------------------------------------------+
|  CBSE - Complete Blockchain Symbolic      |
|         Executor (Rust Edition)           |
+-------------------------------------------+

Building contracts with forge...
[OK] Compiled successfully

Running 2 tests for VaultTest

[EXPLORING] test_balanceNeverNegative
  - Paths explored: 128
  - Constraints solved: 45
  - Time: 0.23s

[EXPLORING] test_depositIncreasesBalance
  - Paths explored: 256  
  - Constraints solved: 89
  - Time: 0.51s`,
            highlight: 'execution'
        },
        {
            id: 'view-results',
            title: '4. View Results',
            description: 'If all properties hold, you get a verification attestation. If not, VeriPro shows a counterexample.',
            code: `+------------------------------------------+
|  VERIFICATION SUCCESSFUL                 |
+------------------------------------------+

Summary: 2 tests, 2 passed, 0 failed

+------------------------------------------+
| ATTESTATION                              |
+------------------------------------------+
| Contract Hash: 0x8f3a...b2c1             |
| Spec Hash:     0x2d4e...9a7f             |
| Prover:        0xeD39...88Fd             |
| Timestamp:     2026-01-12T02:45:00Z      |
| Signature:     0x7c9d...3e8a             |
+------------------------------------------+

Ready to commit on-chain.`,
            highlight: 'result'
        },
        {
            id: 'commit-onchain',
            title: '5. Commit On-Chain',
            description: 'Optionally commit the attestation to the blockchain for permanent, verifiable proof of correctness.',
            code: `// Transaction submitted to AttestationRegistry

Contract: 0x742d...5Ca8 (EVM Registry)
Function: commitAttestation(bytes32,bool,bytes32,bytes)

Parameters:
  resultHash:   0x8f3a...b2c1
  passed:       true
  contractHash: 0x2d4e...9a7f
  signature:    0x7c9d...3e8a

Transaction Hash: 0xab12...cd34
[OK] Confirmed in block #12345678

Your verification is now permanently recorded.`,
            highlight: 'blockchain'
        }
    ];

    useEffect(() => {
        if (isPlaying) {
            const timer = setTimeout(() => {
                if (currentStep < steps.length - 1) {
                    setCurrentStep(currentStep + 1);
                } else {
                    setIsPlaying(false);
                }
            }, 4000);
            return () => clearTimeout(timer);
        }
    }, [isPlaying, currentStep, steps.length]);

    return (
        <div className="bg-zinc-950 rounded-sm border border-zinc-800 overflow-hidden">
            {/* Tutorial header */}
            <div className="flex items-center justify-between p-4 border-b border-zinc-800">
                <div className="flex items-center gap-4">
                    <button
                        onClick={() => setIsPlaying(!isPlaying)}
                        className={`w-10 h-10 rounded-sm border flex items-center justify-center transition-colors ${isPlaying
                            ? 'border-zinc-600 text-zinc-400 hover:text-white'
                            : 'border-zinc-700 text-zinc-400 hover:text-white hover:border-zinc-600'
                            }`}
                    >
                        {isPlaying ? '||' : '>'}
                    </button>
                    <div>
                        <h3 className="font-medium text-white">{steps[currentStep].title}</h3>
                        <p className="text-sm text-zinc-500">{steps[currentStep].description}</p>
                    </div>
                </div>
                <div className="text-sm text-zinc-600">
                    Step {currentStep + 1} of {steps.length}
                </div>
            </div>

            {/* Progress bar */}
            <div className="h-px bg-zinc-800">
                <motion.div
                    className="h-full bg-white"
                    animate={{ width: `${((currentStep + 1) / steps.length) * 100}%` }}
                    transition={{ duration: 0.3 }}
                />
            </div>

            {/* Code display */}
            <div className="relative">
                <AnimatePresence mode="wait">
                    <motion.div
                        key={currentStep}
                        initial={{ opacity: 0, x: 20 }}
                        animate={{ opacity: 1, x: 0 }}
                        exit={{ opacity: 0, x: -20 }}
                        className="p-6"
                    >
                        <pre className="text-sm font-mono overflow-x-auto">
                            <code className="text-zinc-300 leading-relaxed whitespace-pre">
                                {steps[currentStep].code}
                            </code>
                        </pre>
                    </motion.div>
                </AnimatePresence>

                {/* Highlight indicator */}
                <div className="absolute top-4 right-4">
                    <span className="px-2 py-1 text-xs text-zinc-500 border border-zinc-800 rounded-sm">
                        {steps[currentStep].highlight}
                    </span>
                </div>
            </div>

            {/* Step navigation */}
            <div className="flex items-center justify-between p-4 border-t border-zinc-800">
                <button
                    onClick={() => setCurrentStep(Math.max(0, currentStep - 1))}
                    disabled={currentStep === 0}
                    className="px-4 py-2 text-sm text-zinc-500 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                >
                    Previous
                </button>

                <div className="flex gap-2">
                    {steps.map((step, i) => (
                        <button
                            key={step.id}
                            onClick={() => setCurrentStep(i)}
                            className={`w-8 h-8 rounded-sm border flex items-center justify-center text-xs transition-all ${i === currentStep
                                ? 'border-white text-white'
                                : i < currentStep
                                    ? 'border-zinc-700 text-zinc-500'
                                    : 'border-zinc-800 text-zinc-600 hover:border-zinc-700'
                                }`}
                        >
                            {i + 1}
                        </button>
                    ))}
                </div>

                <button
                    onClick={() => setCurrentStep(Math.min(steps.length - 1, currentStep + 1))}
                    disabled={currentStep === steps.length - 1}
                    className="px-4 py-2 text-sm text-zinc-500 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                >
                    Next
                </button>
            </div>
        </div>
    );
}

// ===== CODE BLOCK WITH SYNTAX HIGHLIGHTING =====
function CodeBlock({ code, filename }: { code: string; filename?: string }) {
    const [copied, setCopied] = useState(false);

    const copyCode = () => {
        navigator.clipboard.writeText(code);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    };

    return (
        <div className="rounded-sm border border-zinc-800 overflow-hidden bg-zinc-950">
            {filename && (
                <div className="flex items-center justify-between px-4 py-2 bg-zinc-900 border-b border-zinc-800">
                    <span className="text-xs text-zinc-600 font-mono">{filename}</span>
                    <button
                        onClick={copyCode}
                        className="text-xs text-zinc-600 hover:text-white transition-colors"
                    >
                        {copied ? 'Copied' : 'Copy'}
                    </button>
                </div>
            )}
            <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-zinc-400 font-mono leading-relaxed">{code}</code>
            </pre>
        </div>
    );
}

// ===== PROPERTY TYPE CARD =====
function PropertyCard({ title, description, example }: {
    title: string;
    description: string;
    example: string;
}) {
    const [isExpanded, setIsExpanded] = useState(false);

    return (
        <motion.div
            layout
            className="rounded-sm border border-zinc-800 overflow-hidden cursor-pointer transition-colors hover:border-zinc-700"
            onClick={() => setIsExpanded(!isExpanded)}
        >
            <div className="p-4 flex items-start gap-4">
                <div className="w-8 h-8 rounded-sm border border-zinc-800 flex items-center justify-center text-xs text-zinc-600 flex-shrink-0">
                    {title.charAt(0)}
                </div>
                <div className="flex-1">
                    <div className="flex items-center justify-between">
                        <h4 className="font-medium text-white">{title}</h4>
                        <span className="text-zinc-600 text-sm">{isExpanded ? '-' : '+'}</span>
                    </div>
                    <p className="text-sm text-zinc-500 mt-1">{description}</p>
                </div>
            </div>

            <AnimatePresence>
                {isExpanded && (
                    <motion.div
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: 'auto', opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        className="border-t border-zinc-800"
                    >
                        <CodeBlock code={example} filename="Example" />
                    </motion.div>
                )}
            </AnimatePresence>
        </motion.div>
    );
}

// ===== MAIN DOCUMENTATION PAGE =====
export default function Documentation() {
    const [activeSection, setActiveSection] = useState('overview');
    const [sidebarOpen, setSidebarOpen] = useState(true);
    const mainRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const handleScroll = () => {
            if (!mainRef.current) return;

            const scrollPosition = mainRef.current.scrollTop + 150;

            for (const section of sections) {
                const element = document.getElementById(section.id);
                if (element) {
                    const { offsetTop, offsetHeight } = element;
                    if (scrollPosition >= offsetTop && scrollPosition < offsetTop + offsetHeight) {
                        setActiveSection(section.id);
                        break;
                    }
                }
            }
        };

        const mainElement = mainRef.current;
        mainElement?.addEventListener('scroll', handleScroll);
        return () => mainElement?.removeEventListener('scroll', handleScroll);
    }, []);

    const scrollToSection = (id: string) => {
        const element = document.getElementById(id);
        if (element && mainRef.current) {
            const offset = 100;
            mainRef.current.scrollTo({
                top: element.offsetTop - offset,
                behavior: 'smooth'
            });
        }
    };

    return (
        <div className="min-h-screen bg-black text-white">
            {/* Header */}
            <header className="fixed top-0 left-0 right-0 z-50 bg-black border-b border-zinc-900">
                <div className="max-w-7xl mx-auto px-6 py-4 flex items-center justify-between">
                    <Link href="/" className="text-xl font-light tracking-tight hover:text-zinc-400 transition-colors">
                        VERIPRO
                    </Link>
                    <nav className="flex items-center gap-8 text-sm">
                        <Link href="/" className="text-zinc-500 hover:text-white transition-colors">
                            Home
                        </Link>
                        <Link href="/demo" className="text-zinc-500 hover:text-white transition-colors">
                            Demo
                        </Link>
                        <Link href="/docs" className="text-white transition-colors">
                            Docs
                        </Link>
                        <Link
                            href="/app"
                            className="px-4 py-2 border border-zinc-800 rounded-sm text-zinc-400 hover:text-white hover:border-zinc-700 transition-colors"
                        >
                            Open Platform
                        </Link>
                    </nav>
                </div>
            </header>

            <div className="pt-16 flex h-screen">
                {/* Sidebar */}
                <motion.aside
                    initial={{ width: 280 }}
                    animate={{ width: sidebarOpen ? 280 : 60 }}
                    className="fixed left-0 top-16 bottom-0 border-r border-zinc-900 bg-black overflow-hidden z-40"
                >
                    <div className="p-4">
                        <button
                            onClick={() => setSidebarOpen(!sidebarOpen)}
                            className="w-8 h-8 rounded-sm border border-zinc-800 flex items-center justify-center text-zinc-600 hover:text-white hover:border-zinc-700 transition-colors"
                        >
                            {sidebarOpen ? '<' : '>'}
                        </button>
                    </div>

                    {sidebarOpen && (
                        <nav className="px-4">
                            <div className="text-xs uppercase tracking-wider text-zinc-700 mb-4 px-2">
                                Contents
                            </div>
                            <ul className="space-y-1">
                                {sections.map((section, i) => (
                                    <li key={section.id}>
                                        <button
                                            onClick={() => scrollToSection(section.id)}
                                            className={`w-full text-left px-3 py-2 rounded-sm flex items-center gap-3 transition-colors ${activeSection === section.id
                                                ? 'bg-zinc-900 text-white'
                                                : 'text-zinc-500 hover:text-white hover:bg-zinc-900/50'
                                                }`}
                                        >
                                            <span className="text-xs text-zinc-600 w-4">{String(i + 1).padStart(2, '0')}</span>
                                            <span className="text-sm">{section.title}</span>
                                        </button>
                                    </li>
                                ))}
                            </ul>

                            {/* Quick links */}
                            <div className="mt-8 pt-8 border-t border-zinc-900">
                                <div className="text-xs uppercase tracking-wider text-zinc-700 mb-4 px-2">
                                    Links
                                </div>
                                <ul className="space-y-1">
                                    <li>
                                        <a href="#" className="block px-3 py-2 text-sm text-zinc-600 hover:text-white transition-colors">
                                            GitHub Repository
                                        </a>
                                    </li>
                                    <li>
                                        <a href="#" className="block px-3 py-2 text-sm text-zinc-600 hover:text-white transition-colors">
                                            Discord Community
                                        </a>
                                    </li>
                                    <li>
                                        <a href="#" className="block px-3 py-2 text-sm text-zinc-600 hover:text-white transition-colors">
                                            Report Issue
                                        </a>
                                    </li>
                                </ul>
                            </div>
                        </nav>
                    )}
                </motion.aside>

                {/* Main Content */}
                <main
                    ref={mainRef}
                    className="flex-1 overflow-y-auto"
                    style={{ marginLeft: sidebarOpen ? 280 : 60 }}
                >
                    <div className="max-w-4xl mx-auto px-8 py-12">
                        {/* Overview Section */}
                        <section id="overview" className="mb-24">
                            <motion.div
                                initial={{ opacity: 0, y: 20 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ duration: 0.6 }}
                            >
                                <div className="flex items-center gap-3 mb-4">
                                    <span className="px-2 py-1 text-xs text-zinc-600 border border-zinc-800 rounded-sm">
                                        v1.0
                                    </span>
                                    <span className="text-zinc-600 text-sm">Last updated: January 2026</span>
                                </div>

                                <h1 className="text-5xl font-light mb-6 tracking-tight text-white">
                                    VeriPro Documentation
                                </h1>

                                <p className="text-xl text-zinc-400 mb-8 leading-relaxed">
                                    Complete guide to formal verification of smart contracts using symbolic execution.
                                    VeriPro proves your code is correct for <span className="text-white">all possible inputs</span>,
                                    not just the ones you test.
                                </p>

                                {/* Hero Cards */}
                                <div className="grid grid-cols-3 gap-4 mb-12">
                                    {[
                                        { title: 'Symbolic Execution', desc: 'Explore all code paths automatically' },
                                        { title: 'Formal Verification', desc: 'Mathematical proofs of correctness' },
                                        { title: 'On-Chain Attestation', desc: 'Verifiable proof on blockchain' },
                                    ].map((card, i) => (
                                        <motion.div
                                            key={i}
                                            initial={{ opacity: 0, y: 20 }}
                                            animate={{ opacity: 1, y: 0 }}
                                            transition={{ delay: i * 0.1 }}
                                            className="p-6 border border-zinc-800 rounded-sm hover:border-zinc-700 transition-colors"
                                        >
                                            <div className="text-xs text-zinc-600 mb-3">{String(i + 1).padStart(2, '0')}</div>
                                            <h3 className="font-medium text-white mb-1">{card.title}</h3>
                                            <p className="text-sm text-zinc-500">{card.desc}</p>
                                        </motion.div>
                                    ))}
                                </div>

                                {/* Comparison */}
                                <div className="border border-zinc-800 rounded-sm p-6">
                                    <h3 className="font-medium text-white mb-4">Traditional Testing vs Symbolic Execution</h3>
                                    <div className="grid grid-cols-2 gap-6">
                                        <div className="space-y-3">
                                            <div className="flex items-center gap-2 text-zinc-500 text-sm">
                                                Traditional Testing
                                            </div>
                                            <CodeBlock
                                                code={`test(5)   [PASS]
test(10)  [PASS]  
test(100) [PASS]
...
// But what about edge cases?
// test(2^256-1) ??? [FAIL]`}
                                            />
                                        </div>
                                        <div className="space-y-3">
                                            <div className="flex items-center gap-2 text-white text-sm">
                                                Symbolic Execution
                                            </div>
                                            <CodeBlock
                                                code={`test(X) where X in [0, 2^256-1]

// ALL possible values checked
// [OK] Mathematically proven
// [OK] No edge cases missed
// [OK] Counterexamples found`}
                                            />
                                        </div>
                                    </div>
                                </div>
                            </motion.div>
                        </section>

                        {/* AI Security Agent Section */}
                        <section id="ai-security" className="mb-24">
                            <h2 className="text-3xl font-light mb-2 tracking-tight">AI Security Agent</h2>
                            <p className="text-zinc-500 mb-8">
                                Automated vulnerability scanning and specification generation for EVM & BNB Chain contracts
                            </p>

                            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-12">
                                <PropertyCard
                                    title="Autonomous Scanning"
                                    description="Detects critical vulnerabilities in your code instantly, including Reentrancy, Overflow, and Permission issues."
                                    example={`// AI detects vulnerability:
// [CRITICAL] Reentrancy detected in function 'withdraw'
// Recommendation: Use checks-effects-interactions pattern`}
                                />
                                <PropertyCard
                                    title="Spec Generation"
                                    description="Automatically writes formal invariant tests for your contracts, so you don't start from scratch."
                                    example={`// AI generated invariant:
function invariant_solvent() public view {
    assert(token.totalSupply() == address(vault).balance);
}`}
                                />
                                <PropertyCard
                                    title="BNB Chain Support"
                                    description="Specially optimized for Binance Smart Chain contracts, including BEP-20 token standards and BSC-specific opcodes."
                                    example={`// Optimized for BSC Gas mechanics
// Handles BEP-20 Compliance checks`}
                                />
                                <PropertyCard
                                    title="Vulnerability Report"
                                    description="Provides a comprehensive security report with actionable fixes and generated test cases to prove the fix."
                                    example={`// Report Summary:
// - High: 1 (Reentrancy)
// - Medium: 2 (Unchecked Return Value)
// - Low: 0`}
                                />
                            </div>

                            <div className="bg-zinc-900/30 border border-zinc-800 rounded-lg p-6">
                                <h3 className="text-xl font-medium text-white mb-6 flex items-center gap-2">
                                    <svg className="w-5 h-5 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                                    </svg>
                                    How to Conduct an AI Security Scan
                                </h3>
                                <div className="space-y-6">
                                    <div className="flex gap-4">
                                        <div className="w-8 h-8 rounded-full bg-zinc-800 border border-zinc-700 flex items-center justify-center text-sm font-medium text-white shrink-0">1</div>
                                        <div>
                                            <h4 className="text-white font-medium mb-1">Enter Contract Code</h4>
                                            <p className="text-sm text-zinc-400">Navigate to the <strong>&quot;VeriPro AI&quot;</strong> tab in your project dashboard. Paste your Solidity contract code into the editor or select an existing file.</p>
                                        </div>
                                    </div>

                                    <div className="flex gap-4">
                                        <div className="w-8 h-8 rounded-full bg-zinc-800 border border-zinc-700 flex items-center justify-center text-sm font-medium text-white shrink-0">2</div>
                                        <div>
                                            <h4 className="text-white font-medium mb-1">Generate Specifications</h4>
                                            <p className="text-sm text-zinc-400">Click the <span className="text-purple-400 bg-purple-900/20 px-2 py-0.5 rounded border border-purple-900/50 text-xs">✨ Generate Specs</span> button. The AI Agent will analyze your code for vulnerabilities (like Reentrancy, Overflow, Access Control) and automatically write formal verification tests to prove their absence.</p>
                                        </div>
                                    </div>

                                    <div className="flex gap-4">
                                        <div className="w-8 h-8 rounded-full bg-zinc-800 border border-zinc-700 flex items-center justify-center text-sm font-medium text-white shrink-0">3</div>
                                        <div>
                                            <h4 className="text-white font-medium mb-1">Review & Run Verification</h4>
                                            <p className="text-sm text-zinc-400">The generated specs will appear in the &quot;Specification&quot; tab. Review them, then click <strong>&quot;Run Verification&quot;</strong>. The backend engine (CBSE) will mathematically prove if the properties hold true.</p>
                                        </div>
                                    </div>

                                    <div className="flex gap-4">
                                        <div className="w-8 h-8 rounded-full bg-zinc-800 border border-zinc-700 flex items-center justify-center text-sm font-medium text-white shrink-0">4</div>
                                        <div>
                                            <h4 className="text-white font-medium mb-1">Commit On-Chain</h4>
                                            <p className="text-sm text-zinc-400">If verification passes, use the <strong>&quot;Commit Attestation&quot;</strong> button to publish a cryptographic proof of security to the <strong>BNB Smart Chain (Testnet)</strong> registry.</p>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <div className="mt-8 bg-gradient-to-br from-purple-900/20 to-blue-900/20 border border-purple-500/20 rounded-lg p-6">
                                <h3 className="text-xl font-medium text-white mb-4 flex items-center gap-2">
                                    <svg className="w-5 h-5 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
                                    </svg>
                                    On-Chain Security AI Agent Scanner
                                </h3>
                                <p className="text-zinc-400 mb-6">
                                    The On-Chain Security AI Agent Scanner is an autonomous verification layer that continuously monitors deployed contracts on the BNB Smart Chain. It combines:
                                </p>
                                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                                    <div className="bg-black/40 p-4 rounded border border-zinc-800">
                                        <div className="text-purple-400 font-medium mb-2">Automated Analysis</div>
                                        <p className="text-xs text-zinc-500">Automatically detects common vulnerabilities (Reentrancy, Overflow) in deployed bytecode.</p>
                                    </div>
                                    <div className="bg-black/40 p-4 rounded border border-zinc-800">
                                        <div className="text-purple-400 font-medium mb-2">Verifiable Proofs</div>
                                        <p className="text-xs text-zinc-500">Generates mathematical proofs of safety using the CBSE engine and anchors them on-chain.</p>
                                    </div>
                                    <div className="bg-black/40 p-4 rounded border border-zinc-800">
                                        <div className="text-purple-400 font-medium mb-2">Real-Time Alerts</div>
                                        <p className="text-xs text-zinc-500">Updates the security registry instantly when new vulnerabilities are discovered or fixed.</p>
                                    </div>
                                </div>
                            </div>
                        </section>

                        {/* How It Works Section */}
                        <section id="how-it-works" className="mb-24">
                            <h2 className="text-3xl font-light mb-2 tracking-tight">How It Works</h2>
                            <p className="text-zinc-500 mb-8">
                                Understand the verification flow from contract to proof
                            </p>

                            <AnimatedFlowDiagram />

                            {/* Process explanation */}
                            <div className="grid grid-cols-2 gap-6 mt-8">
                                <div className="space-y-4">
                                    <div className="flex items-start gap-4">
                                        <div className="w-8 h-8 rounded-sm border border-zinc-800 flex items-center justify-center text-xs text-zinc-500">01</div>
                                        <div>
                                            <h4 className="font-medium text-white">Input Processing</h4>
                                            <p className="text-sm text-zinc-500">Your contract and specification are parsed and compiled using Forge</p>
                                        </div>
                                    </div>
                                    <div className="flex items-start gap-4">
                                        <div className="w-8 h-8 rounded-sm border border-zinc-800 flex items-center justify-center text-xs text-zinc-500">02</div>
                                        <div>
                                            <h4 className="font-medium text-white">Symbolic Execution</h4>
                                            <p className="text-sm text-zinc-500">CBSE explores all execution paths with symbolic inputs</p>
                                        </div>
                                    </div>
                                </div>
                                <div className="space-y-4">
                                    <div className="flex items-start gap-4">
                                        <div className="w-8 h-8 rounded-sm border border-zinc-800 flex items-center justify-center text-xs text-zinc-500">03</div>
                                        <div>
                                            <h4 className="font-medium text-white">Constraint Solving</h4>
                                            <p className="text-sm text-zinc-500">Z3 SMT solver finds if any path violates assertions</p>
                                        </div>
                                    </div>
                                    <div className="flex items-start gap-4">
                                        <div className="w-8 h-8 rounded-sm border border-zinc-800 flex items-center justify-center text-xs text-zinc-500">04</div>
                                        <div>
                                            <h4 className="font-medium text-white">Attestation</h4>
                                            <p className="text-sm text-zinc-500">Results are signed and can be committed on-chain</p>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </section>

                        {/* Architecture Section */}
                        <section id="architecture" className="mb-24">
                            <h2 className="text-3xl font-light mb-2 tracking-tight">Architecture</h2>
                            <p className="text-zinc-500 mb-8">
                                Explore the system components and how they interact
                            </p>

                            <ArchitectureDiagram />
                        </section>

                        {/* Interactive Tutorial Section */}
                        <section id="tutorial" className="mb-24">
                            <div className="flex items-center justify-between mb-8">
                                <div>
                                    <h2 className="text-3xl font-light mb-2 tracking-tight">Interactive Tutorial</h2>
                                    <p className="text-zinc-500">
                                        Learn VeriPro step by step with this interactive guide
                                    </p>
                                </div>
                                <span className="px-2 py-1 text-xs text-zinc-500 border border-zinc-800 rounded-sm">
                                    Recommended
                                </span>
                            </div>

                            <InteractiveTutorial />
                        </section>

                        {/* Writing Specifications Section */}
                        <section id="writing-specs" className="mb-24">
                            <h2 className="text-3xl font-light mb-2 tracking-tight">Writing Specifications</h2>
                            <p className="text-zinc-500 mb-8">
                                Learn how to express formal properties about your contracts
                            </p>

                            <div className="space-y-4">
                                <PropertyCard
                                    title="Invariants"
                                    description="Properties that must hold in ALL states of the contract"
                                    example={`// An invariant: total supply equals sum of all balances
function invariant_supplyMatchesBalances() public view {
    uint256 sum = 0;
    for (uint i = 0; i < holders.length; i++) {
        sum += balances[holders[i]];
    }
    assertEq(totalSupply, sum);
}`}
                                />

                                <PropertyCard
                                    title="Safety Properties"
                                    description="Bad states that should NEVER be reachable"
                                    example={`// Safety: Only owner can mint
function test_onlyOwnerCanMint(address caller) public {
    vm.assume(caller != owner);
    
    uint256 supplyBefore = token.totalSupply();
    
    vm.prank(caller);
    vm.expectRevert();
    token.mint(caller, 1000);
    
    assertEq(token.totalSupply(), supplyBefore);
}`}
                                />

                                <PropertyCard
                                    title="Functional Correctness"
                                    description="Verify that functions behave exactly as specified"
                                    example={`// Transfer must update balances correctly
function test_transferUpdatesBalances(
    address from, 
    address to, 
    uint256 amount
) public {
    vm.assume(from != to);
    vm.assume(token.balanceOf(from) >= amount);
    
    uint256 fromBefore = token.balanceOf(from);
    uint256 toBefore = token.balanceOf(to);
    
    vm.prank(from);
    token.transfer(to, amount);
    
    assertEq(token.balanceOf(from), fromBefore - amount);
    assertEq(token.balanceOf(to), toBefore + amount);
}`}
                                />

                                <PropertyCard
                                    title="Access Control"
                                    description="Ensure only authorized users can perform sensitive operations"
                                    example={`// Only admin can pause the contract
function test_onlyAdminCanPause(address attacker) public {
    vm.assume(attacker != admin);
    vm.assume(!paused);
    
    vm.prank(attacker);
    vm.expectRevert("Unauthorized");
    contract.pause();
    
    assertFalse(paused);
}`}
                                />
                            </div>
                        </section>

                        {/* API Reference Section */}
                        <section id="api-reference" className="mb-24">
                            <h2 className="text-3xl font-light mb-2 tracking-tight">API Reference</h2>
                            <p className="text-zinc-500 mb-8">
                                Complete reference for VeriPro commands and cheatcodes
                            </p>

                            {/* Cheatcodes */}
                            <div className="mb-12">
                                <h3 className="text-xl font-light mb-4">VM Cheatcodes</h3>
                                <div className="border border-zinc-800 rounded-sm divide-y divide-zinc-800">
                                    {[
                                        { name: 'vm.assume(bool condition)', desc: 'Add path constraint. Path is pruned if condition is false.' },
                                        { name: 'vm.prank(address sender)', desc: 'Set msg.sender for the next call only.' },
                                        { name: 'vm.startPrank(address sender)', desc: 'Set msg.sender for all subsequent calls until stopPrank.' },
                                        { name: 'vm.deal(address who, uint256 amount)', desc: 'Set the ETH balance of an address.' },
                                        { name: 'vm.roll(uint256 blockNumber)', desc: 'Set block.number.' },
                                        { name: 'vm.warp(uint256 timestamp)', desc: 'Set block.timestamp.' },
                                        { name: 'vm.expectRevert()', desc: 'Expect the next call to revert.' },
                                        { name: 'vm.expectRevert(bytes4 selector)', desc: 'Expect revert with specific error selector.' },
                                    ].map((cheat, i) => (
                                        <div key={i} className="p-4 flex items-start gap-4">
                                            <code className="text-sm font-mono text-zinc-300 flex-shrink-0">{cheat.name}</code>
                                            <p className="text-sm text-zinc-500">{cheat.desc}</p>
                                        </div>
                                    ))}
                                </div>
                            </div>

                            {/* CLI Commands */}
                            <div>
                                <h3 className="text-xl font-light mb-4">Command Line Options</h3>
                                <CodeBlock
                                    filename="CLI Reference"
                                    code={`cbse [OPTIONS] <FILE>

OPTIONS:
    --root <PATH>           Project root directory (default: .)
    --match-contract <REGEX> Run tests in contracts matching regex
    --match-test <REGEX>     Run tests matching regex  
    --function <PREFIX>      Run functions with prefix (default: test_|check_|invariant_)
    
    --loop-bound <N>         Maximum loop iterations (default: 2)
    --depth <N>              Maximum execution depth
    --width <N>              Maximum number of paths
    
    --solver <NAME>          SMT solver: z3, yices, cvc5 (default: yices)
    --solver-timeout <MS>    Solver timeout in milliseconds
    
    --prover-mode            Output signed attestation JSON
    --private-key <HEX>      Private key for signing attestations
    
    --verbose, -v            Increase verbosity (can repeat: -vvv)
    --debug                  Enable debug output
    --statistics             Show solver statistics`}
                                />
                            </div>
                        </section>

                        {/* Examples Section */}
                        <section id="examples" className="mb-24">
                            <h2 className="text-3xl font-light mb-2 tracking-tight">Examples</h2>
                            <p className="text-zinc-500 mb-8">
                                Real-world verification examples you can learn from
                            </p>

                            <div className="grid gap-4">
                                {[
                                    {
                                        title: 'ERC20 Token Verification',
                                        desc: 'Verify standard token invariants: supply consistency, transfer correctness, approval handling',
                                        tags: ['Token', 'DeFi', 'Beginner'],
                                        lines: 120
                                    },
                                    {
                                        title: 'Vault Security Properties',
                                        desc: 'Prove that only depositors can withdraw, and funds cannot be locked',
                                        tags: ['DeFi', 'Security', 'Intermediate'],
                                        lines: 85
                                    },
                                    {
                                        title: 'Access Control Verification',
                                        desc: 'Verify role-based permissions and admin functions',
                                        tags: ['Security', 'Access Control'],
                                        lines: 65
                                    },
                                    {
                                        title: 'AMM Swap Invariants',
                                        desc: 'Prove constant product invariant holds across all swaps',
                                        tags: ['DeFi', 'AMM', 'Advanced'],
                                        lines: 200
                                    },
                                ].map((example, i) => (
                                    <motion.div
                                        key={i}
                                        initial={{ opacity: 0, y: 20 }}
                                        animate={{ opacity: 1, y: 0 }}
                                        transition={{ delay: i * 0.1 }}
                                        className="flex items-center justify-between p-6 border border-zinc-800 rounded-sm hover:border-zinc-700 transition-colors cursor-pointer group"
                                    >
                                        <div>
                                            <h4 className="font-medium text-white group-hover:text-zinc-300 transition-colors">
                                                {example.title}
                                            </h4>
                                            <p className="text-sm text-zinc-500 mt-1">{example.desc}</p>
                                            <div className="flex gap-2 mt-3">
                                                {example.tags.map((tag) => (
                                                    <span key={tag} className="px-2 py-0.5 text-xs border border-zinc-800 text-zinc-600 rounded-sm">
                                                        {tag}
                                                    </span>
                                                ))}
                                            </div>
                                        </div>
                                        <div className="text-right">
                                            <div className="text-lg text-zinc-700 group-hover:text-zinc-500 transition-colors">-&gt;</div>
                                            <div className="text-xs text-zinc-700 mt-2">{example.lines} lines</div>
                                        </div>
                                    </motion.div>
                                ))}
                            </div>
                        </section>

                        {/* Footer */}
                        <footer className="border-t border-zinc-900 pt-12 pb-24">
                            <div className="flex items-center justify-between">
                                <div>
                                    <h3 className="font-medium text-white mb-2">Need Help?</h3>
                                    <p className="text-sm text-zinc-500">
                                        Join our community or reach out to the team
                                    </p>
                                </div>
                                <div className="flex gap-4">
                                    <a href="#" className="px-4 py-2 border border-zinc-800 rounded-sm text-sm text-zinc-400 hover:text-white hover:border-zinc-700 transition-colors">
                                        Discord
                                    </a>
                                    <a href="#" className="px-4 py-2 border border-zinc-800 rounded-sm text-sm text-zinc-400 hover:text-white hover:border-zinc-700 transition-colors">
                                        GitHub
                                    </a>
                                    <a href="#" className="px-4 py-2 bg-white text-black rounded-sm text-sm hover:bg-zinc-200 transition-colors">
                                        Contact Support
                                    </a>
                                </div>
                            </div>
                        </footer>
                    </div>
                </main>
            </div>
        </div>
    );
}
