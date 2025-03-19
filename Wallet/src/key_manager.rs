use crate::errors::{Result, WalletError};
use frost_secp256k1::{
    Identifier,
    frost_core::frost::parameters::ThresholdParameters as Parameters,
    frost_core::frost::keys::{KeyPackage, PublicKeyPackage},
    frost_core::frost::dkg::{round1, round2},
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyShare {
    pub id: u16,
    pub signing_key: String,   // Serialized signing key package
    pub verifying_key: String, // Serialized verifying key (public key)
    pub threshold: u16,
    pub total_shares: u16,
}

pub struct KeyManager;

impl KeyManager {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn generate_key_shares(&self, threshold: u16, total_shares: u16) -> Result<HashMap<String, KeyShare>> {
        if threshold > total_shares {
            return Err(WalletError::KeyGeneration(
                "Threshold cannot be greater than total shares".to_string()
            ));
        }
        
        // Initialize RNG
        let mut rng = OsRng;
        
        // Create parameters for FROST DKG
        let params = Parameters::new(threshold, total_shares)
            .map_err(|_| WalletError::KeyGeneration("Invalid parameters".to_string()))?;
        
        // Round 1: Each participant generates commitments
        let mut round1_packages = BTreeMap::new();
        let mut round1_secrets = BTreeMap::new();
        
        for i in 1..=total_shares {
            let identifier = Identifier::try_from(i)
                .map_err(|e| WalletError::KeyGeneration(format!("Invalid identifier: {}", e)))?;
                
            let (secret, package) = round1::commit(&params, identifier, &mut rng)
                .map_err(|e| WalletError::KeyGeneration(format!("Round 1 error: {}", e)))?;
                
            round1_secrets.insert(identifier, secret);
            round1_packages.insert(identifier, package);
        }
        
        // Broadcast round 1 packages (in a real system, this would be network communication)
        // Round 2: Each participant verifies commitments and generates key shares
        let mut key_shares = HashMap::new();
        
        for i in 1..=total_shares {
            let identifier = Identifier::try_from(i)
                .map_err(|e| WalletError::KeyGeneration(format!("Invalid identifier: {}", e)))?;
                
            let round1_secret = round1_secrets.get(&identifier)
                .ok_or_else(|| WalletError::KeyGeneration("Missing round 1 secret".to_string()))?;
                
            let (key_package, public_key_package) = round2::finalize(
                &params,
                identifier,
                round1_secret.clone(),
                &round1_packages,
            ).map_err(|e| WalletError::KeyGeneration(format!("Round 2 error: {}", e)))?;
            
            // Serialize the key material
            let signing_key = serde_json::to_string(&key_package)
                .map_err(|e| WalletError::Serialization(e))?;
                
            let verifying_key_str = serde_json::to_string(&public_key_package)
                .map_err(|e| WalletError::Serialization(e))?;
            
            key_shares.insert(
                i.to_string(),
                KeyShare {
                    id: i,
                    signing_key,
                    verifying_key: verifying_key_str,
                    threshold,
                    total_shares,
                },
            );
        }
        
        Ok(key_shares)
    }
    
    pub fn save_key_share(&self, share_id: &str, share: &KeyShare) -> Result<()> {
        let share_path = format!("share_{}.json", share_id);
        let share_str = serde_json::to_string(share)?;
        fs::write(share_path, share_str)?;
        Ok(())
    }
    
    pub fn load_key_share(&self, share_id: &str) -> Result<KeyShare> {
        let share_path = format!("share_{}.json", share_id);
        if !Path::new(&share_path).exists() {
            return Err(WalletError::IO(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Key share file not found: {}", share_path),
            )));
        }
        
        let share_str = fs::read_to_string(share_path)?;
        let share: KeyShare = serde_json::from_str(&share_str)?;
        Ok(share)
    }
} 