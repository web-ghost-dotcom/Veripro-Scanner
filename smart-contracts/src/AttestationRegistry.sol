// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title AttestationRegistry
 * @notice Stores verification results from CBSE Provers.
 */
contract AttestationRegistry {
    // Events
    event VerificationAttested(
        bytes32 indexed resultHash,
        address indexed prover,
        bool passed,
        bytes32 contractHash
    );

    event ProverAuthorized(address prover);
    event ProverRevoked(address prover);

    // State
    mapping(address => bool) public isProver;
    address public owner;

    constructor() {
        owner = msg.sender;
    }

    modifier onlyOwner() {
        require(msg.sender == owner, "Only owner");
        _;
    }

    function setProver(address prover, bool status) external onlyOwner {
        isProver[prover] = status;
        if (status) {
            emit ProverAuthorized(prover);
        } else {
            emit ProverRevoked(prover);
        }
    }

    /**
     * @notice Records a verification result on-chain.
     * @dev The signature must be from an authorized Prover.
     * @param resultHash The hash of the VerificationResult JSON.
     * @param passed Whether the verification passed.
     * @param contractHash Hash of the verified bytecode.
     * @param v ECDSA signature v
     * @param r ECDSA signature r
     * @param s ECDSA signature s
     */
    function commitAttestation(
        bytes32 resultHash,
        bool passed,
        bytes32 contractHash,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external {
        // Recover signer from the hash and signature
        // Note: Assumes Rust prover signed the resultHash directly (not EIP-191 prefixed)
        address signer = ecrecover(resultHash, v, r, s);

        require(signer != address(0), "Invalid signature");
        require(isProver[signer], "Signer is not an authorized prover");

        emit VerificationAttested(resultHash, signer, passed, contractHash);
    }
}
