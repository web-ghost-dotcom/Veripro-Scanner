'use client';

import { motion } from 'framer-motion';

export default function SpecificationExample() {
    const spec = `/// @notice Verify token balance invariants
/// @custom:halmos --solver-timeout-assertion 60
contract TokenInvariantTest {
    
    function check_balance_never_exceeds_supply(
        uint256 balance,
        uint256 totalSupply
    ) public pure {
        // Symbolic assertion
        assert(balance <= totalSupply);
    }
    
    function check_transfer_preserves_sum(
        uint256 balanceA,
        uint256 balanceB,
        uint256 amount
    ) public pure {
        uint256 initialSum = balanceA + balanceB;
        
        // After transfer
        uint256 newA = balanceA - amount;
        uint256 newB = balanceB + amount;
        
        assert(newA + newB == initialSum);
    }
}`;

    return (
        <motion.div
            initial={{ opacity: 0, x: -20 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.8 }}
            className="relative rounded-sm border border-zinc-800 bg-black p-6 font-mono text-xs overflow-hidden"
        >
            <div className="absolute top-3 right-3 text-zinc-600 text-xs uppercase tracking-wider">
                Solidity Spec
            </div>
            <pre className="text-zinc-400 overflow-x-auto">
                <code>{spec}</code>
            </pre>
        </motion.div>
    );
}
