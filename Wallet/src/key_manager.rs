use crate::errors::{Result, WalletError};
use rand::{rngs::OsRng, Rng};
use secp256k1::{Secp256k1, SecretKey, PublicKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use sha2::{Sha256, Digest};

// Define a point on the elliptic curve
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EcPoint {
    pub x: String, // Hex-encoded x coordinate
    pub y: String, // Hex-encoded y coordinate
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyShare {
    pub id: u16,
    pub share_polynomial: Vec<String>,  // Coefficients used for this participant (hex-encoded)
    pub share_commit: Vec<EcPoint>,     // Public commitments to the coefficients
    pub public_key: String,             // Hex-encoded common public key
    pub verification_shares: Vec<EcPoint>, // Public verification shares for each party
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
        
        // Create a secp256k1 context
        let secp = Secp256k1::new();
        let mut rng = OsRng;
        
        // Initialize result map
        let mut key_shares = HashMap::new();
        
        // Each party generates their secret polynomial with coefficients a_i,0 to a_i,t-1
        // where a_i,0 is their secret share
        let mut verification_points = vec![Vec::new(); total_shares as usize + 1];
        
        // Step 1: Each party generates their polynomial coefficients
        for i in 1..=total_shares {
            // Generate t random coefficients for f_i(x) = a_i,0 + a_i,1*x + ... + a_i,t-1*x^(t-1)
            let mut coefficients = Vec::with_capacity(threshold as usize);
            let mut commitments = Vec::with_capacity(threshold as usize);
            
            for _ in 0..threshold {
                // Generate random coefficient
                let coeff = SecretKey::new(&mut rng);
                
                // Create public commitment to the coefficient (g^coeff)
                let commit = PublicKey::from_secret_key(&secp, &coeff);
                
                // Convert to our EcPoint format
                let uncompressed = commit.serialize_uncompressed();
                let ec_point = EcPoint {
                    x: hex::encode(&uncompressed[1..33]),
                    y: hex::encode(&uncompressed[33..65]),
                };
                
                // Store coefficient and commitment
                coefficients.push(hex::encode(coeff.secret_bytes()));
                commitments.push(ec_point);
            }
            
            // Step 2: Each party computes their verification shares v_ij for all parties
            // For each other party, evaluate the polynomial at their index
            for j in 1..=total_shares {
                let mut eval = SecretKey::new(&mut rng);  // This would actually be calculated, not random
                let eval_point = PublicKey::from_secret_key(&secp, &eval);
                
                let uncompressed = eval_point.serialize_uncompressed();
                let ec_point = EcPoint {
                    x: hex::encode(&uncompressed[1..33]),
                    y: hex::encode(&uncompressed[33..65]),
                };
                
                if verification_points[j as usize].len() < total_shares as usize {
                    verification_points[j as usize].push(ec_point);
                }
            }
            
            // Calculate the party's public key share (g^a_i,0)
            let secret_share = SecretKey::from_slice(&hex::decode(coefficients[0].clone()).unwrap())
                .map_err(|e| WalletError::KeyGeneration(format!("Invalid secret share: {}", e)))?;
            let public_share = PublicKey::from_secret_key(&secp, &secret_share);
            
            // Create the key share for this party
            key_shares.insert(
                i.to_string(),
                KeyShare {
                    id: i,
                    share_polynomial: coefficients,
                    share_commit: commitments,
                    // This would be the common public key Y = Σ y_i
                    public_key: hex::encode(public_share.serialize()),
                    verification_shares: verification_points[i as usize].clone(),
                    threshold,
                    total_shares,
                },
            );
        }
        
        // In a real implementation, parties would exchange commitments and verify them
        // For this demo, we'll simulate success and assign the same public key to all shares
        
        // Generate a common public key (in real protocol, this would be the sum of all public shares)
        let dummy_pk = SecretKey::new(&mut rng);
        let common_public_key = PublicKey::from_secret_key(&secp, &dummy_pk);
        let common_pk_hex = hex::encode(common_public_key.serialize());
        
        // Update all shares with the common public key
        for (_, share) in key_shares.iter_mut() {
            share.public_key = common_pk_hex.clone();
        }
        
        Ok(key_shares)
    }
    
    // Generate a random challenge for the signing protocol
    pub fn generate_challenge(&self, message: &[u8], r_point: &EcPoint) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(message);
        hasher.update(hex::decode(&r_point.x).unwrap());
        hasher.update(hex::decode(&r_point.y).unwrap());
        hasher.finalize().to_vec()
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