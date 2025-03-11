use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use secp256k1::{SecretKey, PublicKey};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Party {
    id: u32,
    secret_share: Scalar,
    public_key: PublicKey,
    threshold: u32,
    total_parties: u32,
}

#[derive(Debug)]
pub struct ShareGeneration {
    coefficients: Vec<Scalar>,
    threshold: u32,
}

impl ShareGeneration {
    pub fn new(secret: &SecretKey, threshold: u32) -> Self {
        let mut rng = OsRng;
        let mut coefficients = Vec::with_capacity(threshold as usize);
        
        // Convert secret key to scalar
        let secret_scalar = Scalar::from_bytes_mod_order(secret.secret_bytes());
        coefficients.push(secret_scalar);
        
        // Generate random coefficients for the polynomial
        for _ in 1..threshold {
            coefficients.push(Scalar::random(&mut rng));
        }
        
        Self {
            coefficients,
            threshold,
        }
    }
    
    pub fn evaluate(&self, x: u32) -> Scalar {
        let x_scalar = Scalar::from(x as u64);
        let mut result = self.coefficients[0];
        let mut power = x_scalar;
        
        for coeff in self.coefficients.iter().skip(1) {
            result += *coeff * power;
            power *= x_scalar;
        }
        
        result
    }
}