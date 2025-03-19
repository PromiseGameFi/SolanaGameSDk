use crate::errors::{Result, WalletError};
use crate::key_manager::{KeyManager, KeyShare};
use frost_secp256k1::{
    Identifier,
    frost_core::frost::parameters::ThresholdParameters as Parameters,
    frost_core::frost::keys::{KeyPackage, PublicKeyPackage},
    frost_core::frost::round1,
    frost_core::frost::round2,
    frost_core::frost::SigningPackage,
    frost_core::frost::Signature,
    frost_core::frost::aggregate,
};
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};
use std::collections::{BTreeMap, HashMap};
use hex;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PartialSignature {
    pub share_id: String,
    pub commitment: String,      // Serialized commitment
    pub response: String,        // Serialized signing response
    pub threshold: u16,
    pub total_shares: u16,
}

pub struct Signer {
    key_manager: KeyManager,
}

impl Signer {
    pub fn new() -> Self {
        Self {
            key_manager: KeyManager::new(),
        }
    }
    
    // Create a partial signature - WITHOUT reconstructing the private key
    pub fn create_partial_signature(&self, key_share: &KeyShare, message: &[u8]) -> Result<PartialSignature> {
        // Hash the message
        let mut hasher = Sha256::new();
        hasher.update(message);
        let message_hash = hasher.finalize();
        
        // Deserialize the signing key package
        let key_package: KeyPackage = serde_json::from_str(&key_share.signing_key)
            .map_err(|e| WalletError::Serialization(e))?;
            
        // Get the participant's identifier
        let identifier = Identifier::try_from(key_share.id)
            .map_err(|e| WalletError::Signing(format!("Invalid identifier: {}", e)))?;
        
        // Initialize RNG
        let mut rng = OsRng;
        
        // Round 1: Generate nonce and commitment
        let (nonce, commitment) = round1::commit(
            &key_package.secret_share(),
            &mut rng,
        );
        
        // In a real implementation, commitments would be exchanged with other participants
        // Here we simulate a single-party partial signature
        let mut commitments = BTreeMap::new();
        commitments.insert(identifier, commitment.clone());
        
        // Create a signing package
        let signing_package = SigningPackage::new(
            commitments.clone(),
            &message_hash,
        );
        
        // Round 2: Generate partial signature
        let signature_share = round2::sign(
            &signing_package,
            &nonce,
            &key_package,
        ).map_err(|e| WalletError::Signing(format!("Signing error: {}", e)))?;
        
        // Serialize the commitment and signature share
        let commitment_str = serde_json::to_string(&commitment)
            .map_err(|e| WalletError::Serialization(e))?;
            
        let response_str = serde_json::to_string(&signature_share)
            .map_err(|e| WalletError::Serialization(e))?;
        
        Ok(PartialSignature {
            share_id: key_share.id.to_string(),
            commitment: commitment_str,
            response: response_str,
            threshold: key_share.threshold,
            total_shares: key_share.total_shares,
        })
    }
    
    // Combine partial signatures - WITHOUT reconstructing the private key
    pub fn combine_signatures(
        &self,
        partial_signatures: &[PartialSignature],
        message: &[u8],
        verifying_key_str: &str,
    ) -> Result<Vec<u8>> {
        // Check if we have enough partial signatures
        if partial_signatures.is_empty() {
            return Err(WalletError::Threshold("No partial signatures provided".to_string()));
        }
        
        // Get threshold from the first partial signature
        let threshold = partial_signatures[0].threshold;
        
        // Verify all partial signatures have the same threshold
        for sig in partial_signatures {
            if sig.threshold != threshold {
                return Err(WalletError::Threshold("Mismatched thresholds in partial signatures".to_string()));
            }
        }
        
        // Check if we have enough partial signatures
        if partial_signatures.len() < threshold as usize {
            return Err(WalletError::Threshold(format!(
                "Not enough partial signatures: got {}, need {}",
                partial_signatures.len(),
                threshold
            )));
        }
        
        // Hash the message
        let mut hasher = Sha256::new();
        hasher.update(message);
        let message_hash = hasher.finalize();
        
        // Collect commitments and responses
        let mut commitments = BTreeMap::new();
        let mut responses = BTreeMap::new();
        
        for sig in partial_signatures {
            let id = Identifier::try_from(sig.share_id.parse::<u16>().unwrap())
                .map_err(|e| WalletError::Signing(format!("Invalid identifier: {}", e)))?;
                
            let commitment = serde_json::from_str(&sig.commitment)
                .map_err(|e| WalletError::Serialization(e))?;
                
            let response = serde_json::from_str(&sig.response)
                .map_err(|e| WalletError::Serialization(e))?;
                
            commitments.insert(id, commitment);
            responses.insert(id, response);
        }
        
        // Create signing package
        let signing_package = SigningPackage::new(
            commitments,
            &message_hash,
        );
        
        // Deserialize the verifying key
        let verifying_key: PublicKeyPackage = serde_json::from_str(verifying_key_str)
            .map_err(|e| WalletError::Serialization(e))?;
        
        // Aggregate partial signatures to create a complete signature
        // This combines partial signatures WITHOUT reconstructing the private key
        let signature = aggregate(
            &signing_package,
            &responses,
            &verifying_key,
        ).map_err(|e| WalletError::Signing(format!("Signature aggregation error: {}", e)))?;
        
        // Convert FROST signature to Ethereum compatible format
        self.convert_frost_to_ethereum_signature(signature)
    }
    
    // Convert FROST Schnorr signature to Ethereum compatible ECDSA signature
    fn convert_frost_to_ethereum_signature(&self, signature: Signature) -> Result<Vec<u8>> {
        // This is a simplified conversion - in a real implementation, we would need a more complex
        // conversion process to map Schnorr signatures to Ethereum's ECDSA format.
        
        // For Ethereum, we need a 65-byte signature (r, s, v)
        let mut sig_bytes = Vec::with_capacity(65);
        
        // Get R and s components from FROST signature
        // Use public methods instead of accessing private fields
        let serialized = signature.serialize();
        
        // FROST signature format for Secp256k1 is typically 64 bytes (R_x || s)
        // We'll add a recovery byte for Ethereum
        sig_bytes.extend_from_slice(&serialized[0..32]); // R component (r)
        sig_bytes.extend_from_slice(&serialized[32..64]); // s component
        sig_bytes.push(0); // recovery ID (placeholder)
        
        // Note: This signature won't verify on Ethereum without proper conversion
        // A real implementation would use advanced techniques to convert between signature schemes
        
        Ok(sig_bytes)
    }
} 