use crate::errors::{Result, WalletError};
use crate::key_manager::KeyShare;
use frost_secp256k1::{Identifier, KeyPackage, Parameters, SigningCommitment, SigningResponse, SigningPackage, Signature};
use rand::rngs::OsRng;
use secp256k1::{Message, PublicKey};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

pub struct Signer {
    params: Parameters,
}

impl Signer {
    pub fn new(threshold: u16, total_shares: u16) -> Self {
        Self {
            params: Parameters { t: threshold, n: total_shares },
        }
    }
    
    pub fn create_partial_signature(&self, key_share: &KeyShare, message: &[u8]) -> Result<(String, SigningCommitment, SigningResponse)> {
        let mut rng = OsRng;
        let identifier = Identifier::try_from(key_share.identifier.parse::<u16>().unwrap())
            .map_err(|e| WalletError::Signing(format!("Invalid identifier: {}", e)))?;
        
        // Deserialize key package
        let key_package: KeyPackage = serde_json::from_str(&key_share.key_package)
            .map_err(|e| WalletError::Serialization(e))?;
        
        // Hash the message (in a real implementation you would use the transaction hash)
        let mut hasher = Sha256::new();
        hasher.update(message);
        let message_hash = hasher.finalize();
        
        // Generate the nonce commitment for this participant
        let (nonce, commitment) = frost_secp256k1::round1::commit(
            &mut rng,
            identifier,
            &key_package,
        ).map_err(|e| WalletError::Signing(format!("Round 1 error: {}", e)))?;
        
        // Simulate collecting commitments from all required participants (t of them)
        // In a real system these would be received from other participants
        let signing_package = SigningPackage::new(
            // We'd collect commitments from other participants here
            HashMap::from([(identifier, commitment.clone())]),
            Message::from_slice(&message_hash).unwrap(),
        );
        
        // Generate the partial signature
        let signature_share = frost_secp256k1::round2::sign(
            &signing_package,
            nonce,
            &key_package,
        ).map_err(|e| WalletError::Signing(format!("Round 2 error: {}", e)))?;
        
        Ok((identifier.to_string(), commitment, signature_share))
    }
    
    pub fn combine_signatures(
        &self, 
        signing_package: SigningPackage,
        signature_shares: HashMap<Identifier, SigningResponse>,
    ) -> Result<Signature> {
        // Combine the signature shares to get the final signature
        frost_secp256k1::aggregate(&signing_package, &signature_shares)
            .map_err(|e| WalletError::Signing(format!("Signature aggregation error: {}", e)))
    }
} 