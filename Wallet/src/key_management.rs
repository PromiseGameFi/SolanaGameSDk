use anyhow::{anyhow, Result};
use curv::elliptic::curves::{secp256_k1::Secp256k1, Point, Scalar};
use k256::ecdsa::{SigningKey, VerifyingKey};
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::LocalKey;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::{
    party_i::{KeyGenBroadcastMessage1, KeyGenDecommitMessage1, Keys},
    state_machine::keygen::{Keygen, LocalKeyOutput},
};
use rand::rngs::OsRng;
use round_based::Msg;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a share of the distributed key
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyShare {
    pub index: u16,
    pub local_key: LocalKey<Secp256k1>,
}

impl KeyShare {
    /// Convert the key share to a hex string for storage
    pub fn to_hex(&self) -> String {
        let serialized = serde_json::to_string(&self).unwrap();
        hex::encode(serialized.as_bytes())
    }
    
    /// Create a key share from a hex string
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)?;
        let string_data = String::from_utf8(bytes)?;
        Ok(serde_json::from_str(&string_data)?)
    }
}

/// Manages the threshold key shares
pub struct TssKeyManager;

impl TssKeyManager {
    /// Generate a new set of key shares in a t-of-n threshold scheme
    pub fn generate_shares(n: usize, t: usize) -> Result<(Vec<KeyShare>, Point<Secp256k1>)> {
        if t > n {
            return Err(anyhow!("Threshold cannot be greater than the number of shares"));
        }
        
        // Simulate the distributed key generation
        // In a real application, this would be interactive between parties
        let mut rng = OsRng;
        
        // Initialize data structures to track messages
        let mut keygen_states = Vec::with_capacity(n);
        let mut broadcast1_msgs = Vec::with_capacity(n);
        let mut decommit1_msgs = Vec::with_capacity(n);
        
        // Initialize the key generation state machines for each party
        for i in 1..=n {
            let party_id = i as u16;
            let (state, broadcast1) = Keygen::<Secp256k1>::new(&[1, 2, 3, 4, 5][..n], party_id, t as u16)?;
            keygen_states.push(state);
            broadcast1_msgs.push(Msg {
                sender: party_id,
                receiver: None,  // Broadcast
                body: broadcast1,
            });
        }
        
        // Send out round 1 broadcast messages and process them
        for i in 0..n {
            let next_state = keygen_states[i].handle_incoming(broadcast1_msgs.clone())?;
            
            // Generate decommitment for round 1
            let (next_state, decommit1) = next_state.proceed()?;
            keygen_states[i] = next_state;
            
            decommit1_msgs.push(Msg {
                sender: (i + 1) as u16,
                receiver: None,  // Broadcast
                body: decommit1,
            });
        }
        
        // Process round 2 messages and finalize
        let mut key_shares = Vec::with_capacity(n);
        let mut public_key = None;
        
        for i in 0..n {
            let next_state = keygen_states[i].handle_incoming(decommit1_msgs.clone())?;
            
            // Finalize and get the local key
            let local_key_output = next_state.pick_output()?;
            
            // Store the result
            let key_share = KeyShare {
                index: (i + 1) as u16,
                local_key: local_key_output.local_key,
            };
            
            key_shares.push(key_share);
            
            // All parties should converge to the same public key
            if public_key.is_none() {
                public_key = Some(local_key_output.local_key.public_key().clone());
            }
        }
        
        Ok((key_shares, public_key.unwrap()))
    }
    
    /// Extract the public key from a set of key shares
    pub fn extract_public_key(shares: &[KeyShare]) -> Point<Secp256k1> {
        // Any share has the same public key
        shares[0].local_key.public_key().clone()
    }
}