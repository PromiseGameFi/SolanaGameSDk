use secp256k1::{Secp256k1, SecretKey, PublicKey, Message};
use super::shamir::Share;
use anyhow::Result;

pub struct ThresholdSignature {
    secp: Secp256k1<secp256k1::All>,
    shares: Vec<Share>,
    threshold: u32,
}

impl ThresholdSignature {
    pub fn new(shares: Vec<Share>, threshold: u32) -> Self {
        Self {
            secp: Secp256k1::new(),
            shares,
            threshold,
        }
    }

    pub fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>> {
        // Combine shares to reconstruct secret
        let secret_bytes = Share::reconstruct_secret(&self.shares, self.threshold)?;
        let secret_key = SecretKey::from_slice(&secret_bytes)?;
        
        // Sign message
        let message = Message::from_slice(message)?;
        let signature = self.secp.sign_ecdsa(&message, &secret_key);
        
        Ok(signature.serialize_der().to_vec())
    }
}