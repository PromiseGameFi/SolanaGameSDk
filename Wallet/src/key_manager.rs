use crate::errors::{Result, WalletError};
use rand::{rngs::OsRng, Rng};
use secp256k1::{Secp256k1, SecretKey, PublicKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyShare {
    pub id: u8,
    pub share: String, // Hex-encoded share
    pub public_key: String, // Hex-encoded public key
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeygenConfig {
    pub threshold: u8,
    pub shares: u8,
    pub public_key: String, // Hex-encoded public key
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
    
    pub fn generate_key_shares(&self, threshold: u8, total_shares: u8) -> Result<HashMap<String, KeyShare>> {
        if threshold > total_shares {
            return Err(WalletError::KeyGeneration(
                "Threshold cannot be greater than total shares".to_string()
            ));
        }
        
        // Initialize secp256k1 context
        let secp = Secp256k1::new();
        let mut rng = OsRng;
        
        // Generate a random master private key
        let master_sk = SecretKey::new(&mut rng);
        
        // Get the corresponding public key
        let public_key = PublicKey::from_secret_key(&secp, &master_sk);
        let public_key_hex = hex::encode(public_key.serialize().to_vec());
        
        // Split the private key using Shamir Secret Sharing
        let shares = self.split_secret(&master_sk.secret_bytes(), threshold, total_shares)?;
        
        // Convert shares to KeyShare objects
        let mut key_shares = HashMap::new();
        for (id, share) in shares {
            let share_hex = hex::encode(share);
            key_shares.insert(
                id.to_string(),
                KeyShare {
                    id,
                    share: share_hex,
                    public_key: public_key_hex.clone(),
                },
            );
        }
        
        // Save the config
        let config = KeygenConfig {
            threshold,
            shares: total_shares,
            public_key: public_key_hex,
        };
        
        let config_str = serde_json::to_string(&config)?;
        fs::write(&self.config_path, config_str)?;
        
        Ok(key_shares)
    }
    
    // Split a secret using Shamir's Secret Sharing Scheme
    fn split_secret(&self, secret: &[u8; 32], threshold: u8, shares: u8) -> Result<HashMap<u8, Vec<u8>>> {
        if threshold < 2 || threshold > shares {
            return Err(WalletError::KeyGeneration("Invalid threshold".to_string()));
        }
        
        let mut rng = OsRng;
        let mut result = HashMap::new();
        
        // Each byte of the secret is split separately
        for byte_idx in 0..32 {
            // Generate random coefficients for the polynomial
            // The constant term is the secret byte
            let mut coefficients = vec![secret[byte_idx]];
            
            // Generate random coefficients for the polynomial (degree t-1)
            for _ in 1..threshold {
                coefficients.push(rng.r#gen());
            }
            
            // Evaluate the polynomial for each share
            for x in 1..=shares {
                // Evaluate polynomial at point x
                let mut y = coefficients[0]; // Start with the constant term
                let mut x_pow = x; // x^1
                
                for coef in coefficients.iter().skip(1) {
                    // Add coef * x^i to y
                    y = y.wrapping_add(coef.wrapping_mul(x_pow));
                    x_pow = x_pow.wrapping_mul(x); // x^(i+1)
                }
                
                // Store or append the result
                if !result.contains_key(&x) {
                    result.insert(x, vec![y]);
                } else {
                    result.get_mut(&x).unwrap().push(y);
                }
            }
        }
        
        Ok(result)
    }
    
    // Reconstruct a secret from shares using Lagrange interpolation
    pub fn reconstruct_secret(&self, shares: &HashMap<u8, Vec<u8>>, threshold: u8) -> Result<[u8; 32]> {
        if shares.len() < threshold as usize {
            return Err(WalletError::Threshold(format!(
                "Not enough shares: got {}, need {}",
                shares.len(),
                threshold
            )));
        }
        
        let mut secret = [0u8; 32];
        
        // Reconstruct each byte of the secret separately
        for byte_idx in 0..32 {
            let mut result = 0u8;
            
            // Build a list of (x, y) points for this byte index
            let points: Vec<(u8, u8)> = shares
                .iter()
                .take(threshold as usize)
                .map(|(&x, share)| (x, share[byte_idx]))
                .collect();
            
            // Apply Lagrange interpolation
            for i in 0..points.len() {
                let xi = points[i].0;
                let yi = points[i].1;
                
                let mut numerator = 1u8;
                let mut denominator = 1u8;
                
                for j in 0..points.len() {
                    if i != j {
                        let xj = points[j].0;
                        
                        // Calculate (x_j) / (x_i - x_j) for the Lagrange basis polynomial
                        // We're computing in a finite field, so we need to be careful with division
                        let xj_wrapped = xj;
                        let xi_minus_xj = xi.wrapping_sub(xj);
                        
                        numerator = numerator.wrapping_mul(xj_wrapped);
                        denominator = denominator.wrapping_mul(xi_minus_xj);
                    }
                }
                
                // Calculate the modular multiplicative inverse of denominator
                let inverse = self.mod_inverse(denominator, 251); // Using prime 251 for GF(251)
                let term = yi.wrapping_mul(numerator).wrapping_mul(inverse);
                result = result.wrapping_add(term);
            }
            
            secret[byte_idx] = result;
        }
        
        Ok(secret)
    }
    
    // Calculate modular multiplicative inverse using Extended Euclidean Algorithm
    fn mod_inverse(&self, a: u8, m: u8) -> u8 {
        let mut a = a as i16;
        let m = m as i16;
        let mut m0 = m;
        let mut y = 0;
        let mut x = 1;
        
        if m == 1 {
            return 0;
        }
        
        while a > 1 {
            // q is quotient
            let q = a / m0;
            let t = m0;
            
            // m is remainder now, process same as Euclid's algo
            m0 = a % m0;
            a = t;
            let t = y;
            
            // Update y and x
            y = x - q * y;
            x = t;
        }
        
        // Make x positive
        if x < 0 {
            x += m;
        }
        
        x as u8
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