use k256::ecdsa::{
    signature::hazmat::PrehashSigner,
    RecoveryId, Signature, SigningKey, VerifyingKey,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationResult {
    pub passed: bool,
    pub contract_bytecode_hash: String, // keccak256 of the runtime bytecode
    pub spec_hash: String,              // keccak256 of the test content
    pub timestamp: u64,
    pub details: String, // JSON summary of execution
}

impl VerificationResult {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn hash(&self) -> String {
        let json = self.to_json();
        let mut hasher = Keccak256::new();
        hasher.update(json.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationAttestation {
    pub verifier_version: String, // e.g., "cbse-v0.1.0"
    pub result_hash: String,      // keccak256(serde_json::to_string(VerificationResult))
    pub prover_address: String,   // 0x... address of the prover
    #[serde(with = "hex")]
    pub signature: Vec<u8>, // Signature bytes (65 bytes: r + s + v)
    pub payload: VerificationResult,
}

impl VerificationAttestation {
    pub fn sign(
        result: VerificationResult,
        private_key_hex: &str,
        verifier_version: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let result_hash = result.hash();

        let clean_key = private_key_hex.trim_start_matches("0x");
        let signing_key = SigningKey::from_slice(&hex::decode(clean_key)?)?;
        let verifying_key = signing_key.verifying_key();

        let hash_bytes = hex::decode(&result_hash)?;

        // Sign with recovery id for Ethereum compatibility
        let (signature, recovery_id) = signing_key
            .sign_prehash_recoverable(&hash_bytes)
            .map_err(|e| format!("Failed to sign: {}", e))?;

        // Build 65-byte signature: r (32) + s (32) + v (1)
        let mut signature_bytes = signature.to_vec(); // 64 bytes (r + s)
        
        // Add recovery id as v (27 or 28 for Ethereum)
        let v = recovery_id.to_byte() + 27;
        signature_bytes.push(v);

        // Derive address
        let derived_address = eth_address_from_pubkey(&verifying_key);

        Ok(Self {
            verifier_version,
            result_hash,
            prover_address: derived_address,
            signature: signature_bytes,
            payload: result,
        })
    }
}

fn eth_address_from_pubkey(pubkey: &VerifyingKey) -> String {
    let encoded = pubkey.to_encoded_point(false);
    let encoded_bytes = encoded.as_bytes();
    // Skip the first byte (tag) if it exists (0x04 for uncompressed)
    let hash = Keccak256::digest(&encoded_bytes[1..]);
    let address_bytes = &hash[12..];
    format!("0x{}", hex::encode(address_bytes))
}
