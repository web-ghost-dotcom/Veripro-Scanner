'use client';

import { motion } from 'framer-motion';

export default function VerificationOutput() {
    return (
        <motion.div
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: true }}
            transition={{ duration: 0.8 }}
            className="grid grid-cols-1 md:grid-cols-2 gap-4"
        >
            {/* Success Output */}
            <div className="border border-zinc-800 bg-black p-6 font-mono text-xs">
                <div className="flex items-center gap-2 mb-4">
                    <div className="w-2 h-2 rounded-full bg-green-500" />
                    <span className="text-zinc-500 uppercase text-xs tracking-wider">Verification Passed</span>
                </div>
                <pre className="text-zinc-400 space-y-1">
                    <div className="text-green-400">✓ check_balance_never_exceeds_supply</div>
                    <div className="text-zinc-600">  Explored: 256 paths</div>
                    <div className="text-zinc-600">  Time: 42ms</div>
                    <div className="text-zinc-600">  Solver calls: 128</div>
                    <div className="text-green-400 mt-3">✓ check_transfer_preserves_sum</div>
                    <div className="text-zinc-600">  Explored: 512 paths</div>
                    <div className="text-zinc-600">  Time: 67ms</div>
                    <div className="text-zinc-600">  Solver calls: 256</div>
                </pre>
            </div>

            {/* Counterexample Output */}
            <div className="border border-zinc-800 bg-black p-6 font-mono text-xs">
                <div className="flex items-center gap-2 mb-4">
                    <div className="w-2 h-2 rounded-full bg-red-500" />
                    <span className="text-zinc-500 uppercase text-xs tracking-wider">Counterexample Found</span>
                </div>
                <pre className="text-zinc-400 space-y-1">
                    <div className="text-red-400">✗ check_overflow_protection</div>
                    <div className="text-zinc-600">  Assertion violated at line 42</div>
                    <div className="text-zinc-600 mt-2">  Counterexample:</div>
                    <div className="text-zinc-500 ml-4">amount = 0xffffffff...</div>
                    <div className="text-zinc-500 ml-4">balance = 0x00000001</div>
                    <div className="text-zinc-600 mt-2">  Path condition:</div>
                    <div className="text-zinc-500 ml-4">(and (= amount MAX_UINT)</div>
                    <div className="text-zinc-500 ml-4">     (&gt; balance 0))</div>
                </pre>
            </div>

            {/* Constraint Solving Stats */}
            <div className="border border-zinc-800 bg-black p-6 font-mono text-xs">
                <div className="flex items-center gap-2 mb-4">
                    <div className="w-2 h-2 rounded-full bg-blue-500" />
                    <span className="text-zinc-500 uppercase text-xs tracking-wider">Solver Statistics</span>
                </div>
                <pre className="text-zinc-400 space-y-1">
                    <div className="text-zinc-500">SMT Queries: 384</div>
                    <div className="text-zinc-500">Satisfiable: 341</div>
                    <div className="text-zinc-500">Unsatisfiable: 43</div>
                    <div className="text-zinc-500">Total time: 1.2s</div>
                    <div className="text-zinc-600 mt-3">Optimizations:</div>
                    <div className="text-zinc-500 ml-4">• Cache hits: 89%</div>
                    <div className="text-zinc-500 ml-4">• Constraint simplification: On</div>
                    <div className="text-zinc-500 ml-4">• Incremental solving: Enabled</div>
                </pre>
            </div>

            {/* Memory Trace */}
            <div className="border border-zinc-800 bg-black p-6 font-mono text-xs">
                <div className="flex items-center gap-2 mb-4">
                    <div className="w-2 h-2 rounded-full bg-purple-500" />
                    <span className="text-zinc-500 uppercase text-xs tracking-wider">Memory Trace</span>
                </div>
                <pre className="text-zinc-400 space-y-1">
                    <div className="text-purple-400">0x00: PUSH1 0x60</div>
                    <div className="text-purple-400">0x02: PUSH1 0x40</div>
                    <div className="text-purple-400">0x04: MSTORE</div>
                    <div className="text-zinc-600">  mem[0x40] = 0x60</div>
                    <div className="text-purple-400 mt-2">0x05: CALLDATASIZE</div>
                    <div className="text-purple-400">0x06: ISZERO</div>
                    <div className="text-zinc-600">  Branch: [symbolic]</div>
                    <div className="text-zinc-600">  Explored: both paths</div>
                </pre>
            </div>
        </motion.div>
    );
}
