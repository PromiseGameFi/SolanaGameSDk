use crate::errors::{Result, WalletError};
use crate::key_manager::{KeyManager, KeyShare, EcPoint};
use rand::rngs::OsRng;
use secp256k1::{Secp256k1, SecretKey, PublicKey};
use sha2::{Sha256, Digest};
use hex;

// Commitment to a nonce (First round of signing)
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct NonceCommitment {
    pub share_id: String,
    pub r_commitment: EcPoint,  // Public point R_i = g^k_i
    pub session_id: String,     // Unique session ID for this signing
}

// Partial signature (Second round of signing)
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct PartialSignature {
    pub share_id: String,
    pub r_point: EcPoint,        // Final common R point
    pub s_share: String,         // s_i component
    pub threshold: u16,
    pub total_shares: u16,
    pub session_id: String,      // Same session ID as commitment round
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
    
    // First round: Generate nonce and commitment
    pub fn create_nonce_commitment(&self, key_share: &KeyShare, message: &[u8]) -> Result<NonceCommitment> {
        // Create a unique session ID
        let session_id = self.generate_session_id(message);
        
        // Generate a random nonce k_i
        let secp = Secp256k1::new();
        let mut rng = OsRng;
        let k_i = SecretKey::new(&mut rng);
        
        // Calculate the commitment R_i = g^k_i
        let r_i = PublicKey::from_secret_key(&secp, &k_i);
        
        // Convert to our EcPoint format
        let uncompressed = r_i.serialize_uncompressed();
        let r_point = EcPoint {
            x: hex::encode(&uncompressed[1..33]),
            y: hex::encode(&uncompressed[33..65]),
        };
        
        // Normally this commitment would be broadcast to other participants
        // In a demo, we'll just return it
        
        Ok(NonceCommitment {
            share_id: key_share.id.to_string(),
            r_commitment: r_point,
            session_id: hex::encode(session_id),
        })
    }
    
    // Second round: Create partial signature after collecting commitments
    pub fn create_partial_signature(
        &self, 
        key_share: &KeyShare, 
        message: &[u8],
        commitments: &[NonceCommitment],
        session_id: &str,
    ) -> Result<PartialSignature> {
        if commitments.is_empty() {
            return Err(WalletError::Signing("No commitments provided".to_string()));
        }
        
        // Verify all commitments have the same session ID
        for commit in commitments {
            if commit.session_id != session_id {
                return Err(WalletError::Signing("Inconsistent session IDs".to_string()));
            }
        }
        
        // In a real MPC protocol, each party would verify the commitments
        // and compute a common R = Σ R_i
        
        // For demo, we'll use the first commitment's R value as the common R
        let r_point = commitments[0].r_commitment.clone();
        
        // Generate a deterministic k value (the protocol would actually use the shared value)
        let nonce_seed = self.derive_nonce_seed(key_share, message, &hex::decode(session_id).unwrap());
        let k_i = self.generate_deterministic_nonce(&nonce_seed);
        
        // Calculate the challenge e = H(m || R)
        let challenge = self.key_manager.generate_challenge(message, &r_point);
        
        // Compute partial signature s_i = k_i + e * x_i
        // where x_i is the party's secret share (first coefficient of polynomial)
        let s_i = self.compute_partial_signature(key_share, &k_i.secret_bytes(), &challenge);
        
        Ok(PartialSignature {
            share_id: key_share.id.to_string(),
            r_point,
            s_share: hex::encode(s_i),
            threshold: key_share.threshold,
            total_shares: key_share.total_shares,
            session_id: session_id.to_string(),
        })
    }
    
    // Combine partial signatures to create a complete signature
    pub fn combine_signatures(
        &self,
        partial_signatures: &[PartialSignature],
        _message: &[u8],   // Add underscore to indicate intentionally unused
    ) -> Result<Vec<u8>> {
        if partial_signatures.is_empty() {
            return Err(WalletError::Threshold("No signatures provided".to_string()));
        }
        
        // Get parameters from the first signature
        let first_sig = &partial_signatures[0];
        let r_point = &first_sig.r_point;
        let threshold = first_sig.threshold;
        let session_id = &first_sig.session_id;
        
        // Check if we have enough signatures
        if partial_signatures.len() < threshold as usize {
            return Err(WalletError::Threshold(format!(
                "Not enough signatures: have {}, need {}",
                partial_signatures.len(), threshold
            )));
        }
        
        // Verify all signatures belong to the same session
        for sig in partial_signatures {
            if sig.session_id != *session_id || sig.threshold != threshold {
                return Err(WalletError::Signing("Inconsistent signatures".to_string()));
            }
            
            // Also verify they all have the same R point
            if sig.r_point.x != r_point.x || sig.r_point.y != r_point.y {
                return Err(WalletError::Signing("Inconsistent R points".to_string()));
            }
        }
        
        // A proper MPC-TSS would use Lagrange interpolation here
        // We'll simplify by adding s shares (this is not cryptographically correct)
        let mut s_combined = vec![0u8; 32];
        for sig in partial_signatures {
            let s_i = hex::decode(&sig.s_share)
                .map_err(|e| WalletError::Signing(format!("Invalid s share: {}", e)))?;
                
            for i in 0..s_i.len().min(32) {
                s_combined[i] = s_combined[i].wrapping_add(s_i[i]);
            }
        }
        
        // Try both recovery IDs since we can't determine it directly
        self.convert_frost_to_ethereum_signature(r_point, &s_combined)
    }
    
    // Generate a unique session ID
    fn generate_session_id(&self, message: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(message);
        // Optionally add something unique to this transaction but common to all parties
        // For example: hasher.update(transaction_nonce.to_be_bytes());
        hasher.finalize().to_vec()
    }
    
    // Derive a nonce seed deterministically
    fn derive_nonce_seed(&self, share: &KeyShare, message: &[u8], session_id: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(hex::decode(&share.share_polynomial[0]).unwrap()); // Secret share
        hasher.update(message);
        hasher.update(session_id);
        hasher.finalize().to_vec()
    }
    
    // Generate a deterministic nonce from seed
    fn generate_deterministic_nonce(&self, seed: &[u8]) -> SecretKey {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        let nonce_bytes = hasher.finalize();
        
        SecretKey::from_slice(&nonce_bytes)
            .expect("Failed to create nonce from hash")
    }
    
    // Compute partial signature s_i = k_i + e * x_i
    fn compute_partial_signature(&self, share: &KeyShare, k_i: &[u8], challenge: &[u8]) -> Vec<u8> {
        // This implementation is a simplified version
        // A real implementation would use proper EC math
        
        // Parse the party's secret share
        let x_i = hex::decode(&share.share_polynomial[0]).unwrap();
        
        // Simple computation: s_i = k_i + e * x_i
        let mut s_i = vec![0u8; 32];
        for i in 0..32 {
            let e_i = challenge[i % challenge.len()];
            let x_i_val = x_i[i % x_i.len()];
            let k_i_val = k_i[i % k_i.len()];
            
            s_i[i] = k_i_val.wrapping_add(e_i.wrapping_mul(x_i_val));
        }
        
        s_i
    }
    
    // Convert Frost signature to Ethereum signature
    fn convert_frost_to_ethereum_signature(&self, r_point: &EcPoint, s_combined: &[u8]) -> Result<Vec<u8>> {
        // Parse r from the x-coordinate of R
        let r_bytes = hex::decode(&r_point.x)
            .map_err(|e| WalletError::Signing(format!("Invalid R point: {}", e)))?;
        
        // Create signature with recovery ID 0 (will be adjusted by eth.rs)
        let mut signature = Vec::with_capacity(65);
        signature.extend_from_slice(&r_bytes);
        signature.extend_from_slice(s_combined);
        signature.push(0); // Placeholder, will be replaced in eth.rs
        
        Ok(signature)
    }
} 