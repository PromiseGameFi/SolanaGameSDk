use anyhow::{anyhow, Result};
use curv::elliptic::curves::{secp256_k1::Secp256k1, Point, Scalar};
use k256::ecdsa::{RecoveryId, Signature as K256Signature, signature::Signer};
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::{
    party_i::SignatureRecid,
    state_machine::sign::{OfflineStage, SignManual},
};
use rand::rngs::OsRng;
use round_based::Msg;
use sha3::{Digest, Keccak256};
use std::collections::HashMap;

use crate::key_management::KeyShare;
use crate::transaction::EthereumTransaction;

pub struct TssSigner;

impl TssSigner {
    /// Sign an Ethereum transaction using threshold signatures
    pub fn sign_transaction(
        transaction: &EthereumTransaction,
        shares: &[KeyShare],
    ) -> Result<Vec<u8>> {
        // 1. Hash the transaction according to Ethereum rules
        let message_hash = transaction.hash();
        
        // 2. Perform threshold signing
        let signature = Self::sign_message(&message_hash, shares)?;
        
        // 3. Return the signature in Ethereum format (r, s, v)
        Ok(signature)
    }
    
    /// Sign a message hash using threshold signatures
    fn sign_message(message_hash: &[u8], shares: &[KeyShare]) -> Result<Vec<u8>> {
        let mut rng = OsRng;
        
        // Extract participant IDs from the shares
        let mut participant_ids = Vec::new();
        for share in shares {
            participant_ids.push(share.index);
        }
        
        // Initialize offline stage for each party
        let mut states = Vec::new();
        let mut round1_msgs = Vec::new();
        
        for share in shares {
            // Initialize offline signing
            let (state, msg) = OfflineStage::<Secp256k1>::new(
                participant_ids.clone(),
                share.index,
                &share.local_key,
            )?;
            
            states.push(state);
            round1_msgs.push(Msg {
                sender: share.index,
                receiver: None,
                body: msg,
            });
        }
        
        // Process round 1 messages
        let mut round2_msgs = Vec::new();
        for i in 0..shares.len() {
            let next_state = states[i].handle_incoming(round1_msgs.clone())?;
            let (next_state, msg) = next_state.proceed()?;
            states[i] = next_state;
            
            round2_msgs.push(Msg {
                sender: shares[i].index,
                receiver: None,
                body: msg,
            });
        }
        
        // Process round 2 messages
        let mut round3_msgs = Vec::new();
        for i in 0..shares.len() {
            let next_state = states[i].handle_incoming(round2_msgs.clone())?;
            let (next_state, msg) = next_state.proceed()?;
            states[i] = next_state;
            
            round3_msgs.push(Msg {
                sender: shares[i].index,
                receiver: None,
                body: msg,
            });
        }
        
        // Process round 3 messages
        let mut round4_msgs = Vec::new();
        for i in 0..shares.len() {
            let next_state = states[i].handle_incoming(round3_msgs.clone())?;
            let (next_state, msg) = next_state.proceed()?;
            states[i] = next_state;
            
            round4_msgs.push(Msg {
                sender: shares[i].index,
                receiver: None,
                body: msg,
            });
        }
        
        // Process round 4 messages and generate partial signatures
        let mut round5_msgs = Vec::new();
        for i in 0..shares.len() {
            let next_state = states[i].handle_incoming(round4_msgs.clone())?;
            let (next_state, msg) = next_state.proceed()?;
            states[i] = next_state;
            
            round5_msgs.push(Msg {
                sender: shares[i].index,
                receiver: None,
                body: msg,
            });
        }
        
        // Process round 5 messages and finalize
        let mut signing_states = Vec::new();
        
        for i in 0..shares.len() {
            let next_state = states[i].handle_incoming(round5_msgs.clone())?;
            let signing_state = next_state.pick_output()?;
            signing_states.push(signing_state);
        }
        
        // Now we have the offline stage completed, sign the actual message
        // We only need one party to complete the signing
        let message_scalar = {
            let mut hasher = Keccak256::new();
            hasher.update(message_hash);
            let hash = hasher.finalize();
            // Convert the hash to a scalar. In a real implementation, this should be done more carefully.
            let mut hash_arr = [0u8; 32];
            hash_arr.copy_from_slice(&hash);
            Scalar::<Secp256k1>::from_bytes(&hash_arr)?
        };
        
        let completed_signature = signing_states[0].complete(&message_scalar)?;
        
        // Convert to Ethereum signature format
        let r = completed_signature.r.to_bytes().to_vec();
        let s = completed_signature.s.to_bytes().to_vec();
        
        // Calculate recovery ID (v) for Ethereum
        let v = match completed_signature.recid {
            0 => 27u8,
            1 => 28u8,
            _ => return Err(anyhow!("Invalid recovery ID")),
        } + (transaction.chain_id * 2 + 35) as u8; // EIP-155 adjustment
        
        // Concatenate r, s, v
        let mut signature = Vec::with_capacity(65);
        signature.extend_from_slice(&r);
        signature.extend_from_slice(&s);
        signature.push(v);
        
        Ok(signature)
    }
}