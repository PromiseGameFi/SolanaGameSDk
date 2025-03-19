use crate::errors::{Result, WalletError};
use frost_secp256k1::{Identifier, KeyPackage, Parameters, round1, round2, SigningPackage};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyShare {
    pub identifier: String,
    pub key_package: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeygenConfig {
    pub threshold: u16,
    pub shares: u16,
}

pub struct KeyManager {
    config_path: String,
}

impl KeyManager {
    pub fn new(config_path: &str) -> Self {
        Self {
            config_path: config_path.to_string(),
        }
    }
    
    pub fn generate_key_shares(&self, threshold: u16, total_shares: u16) -> Result<HashMap<String, KeyShare>> {
        if threshold > total_shares {
            return Err(WalletError::KeyGeneration(
                "Threshold cannot be greater than total shares".to_string()
            ));
        }
        
        let mut rng = OsRng;
        let params = Parameters { t: threshold, n: total_shares };
        
        let mut shares = HashMap::new();
        
        // Generate key packages for each participant
        for i in 1..=total_shares {
            // Generate round 1 data
            let (round1_secret, round1_public) = round1::commit(&params, Identifier::try_from(i).unwrap(), &mut rng)
                .map_err(|e| WalletError::KeyGeneration(format!("Round 1 error: {}", e)))?;
            
            // Simulate round 2 for all participants (in a real system, these would be distributed)
            let round1_packages: Vec<_> = (1..=total_shares)
                .map(|j| (Identifier::try_from(j).unwrap(), round1_public.clone()))
                .collect();
            
            // Complete round 2 for this participant
            let (key_package, _public_key) = round2::finalize(
                &params,
                Identifier::try_from(i).unwrap(),
                round1_secret,
                &round1_packages,
            ).map_err(|e| WalletError::KeyGeneration(format!("Round 2 error: {}", e)))?;
            
            // Serialize key package for storage
            let key_package_str = serde_json::to_string(&key_package)
                .map_err(|e| WalletError::Serialization(e))?;
            
            shares.insert(
                i.to_string(),
                KeyShare {
                    identifier: i.to_string(),
                    key_package: key_package_str,
                },
            );
        }
        
        // Save the config
        let config = KeygenConfig {
            threshold,
            shares: total_shares,
        };
        
        let config_str = serde_json::to_string(&config)?;
        fs::write(&self.config_path, config_str)?;
        
        Ok(shares)
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
    
    pub fn load_config(&self) -> Result<KeygenConfig> {
        if !Path::new(&self.config_path).exists() {
            return Err(WalletError::IO(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Config file not found: {}", self.config_path),
            )));
        }
        
        let config_str = fs::read_to_string(&self.config_path)?;
        let config: KeygenConfig = serde_json::from_str(&config_str)?;
        Ok(config)
    }
} 