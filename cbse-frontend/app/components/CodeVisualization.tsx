'use client';

import { motion } from 'framer-motion';

export default function CodeVisualization() {
    const codeSnippet = `fn verify_invariant(state: &State) -> Result<()> {
    let constraints = build_constraints(state);
    solver.check_sat(&constraints)?;
    Ok(())
}`;

    return (
        <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8 }}
            className="relative rounded-sm border border-zinc-800 bg-black p-6 font-mono text-sm"
        >
            <div className="absolute top-3 left-3 flex gap-1.5">
                <div className="h-2 w-2 rounded-full bg-zinc-700" />
                <div className="h-2 w-2 rounded-full bg-zinc-700" />
                <div className="h-2 w-2 rounded-full bg-zinc-700" />
            </div>
            <pre className="mt-6 text-zinc-400 overflow-x-auto">
                <code>{codeSnippet}</code>
            </pre>
        </motion.div>
    );
}
