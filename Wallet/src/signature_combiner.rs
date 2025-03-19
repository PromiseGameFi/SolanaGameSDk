use anyhow::{Context, Result};
use curv::arithmetic::Converter;
use curv::elliptic::curves::{Point, Scalar, Secp256k1};
use ethers::core::types::{Bytes, TransactionRequest, U256};
use ethers::utils::rlp::Rlp;
use ethers::utils::{hex, keccak256, parse_ether};
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::{
    SignatureRecid,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use crate::key_splitter::KeyShare;
use crate::partial_signer::{PartialSignature, TransactionData};

#[derive(Serialize, Deserialize, Clone)]
pub struct CombinedSignature {
    pub transaction: TransactionData,
    pub r: String,
    pub s: String,
    pub v: u64,
    pub signed_tx: String,
}

pub fn combine_signatures(
    partial_signatures: Vec<PartialSignature>,
    output_path: &Path,
    share: &KeyShare,
) -> Result<()> {
    if partial_signatures.is_empty() {
        anyhow::bail!("No partial signatures provided");
    }
    
    // Verify that all partial signatures are for the same transaction
    let first_tx = &partial_signatures[0].transaction;
    let message_hash = &partial_signatures[0].message_hash;
    
    for sig in partial_signatures.iter().skip(1) {
        if sig.transaction.to != first_tx.to || 
           sig.transaction.value != first_tx.value ||
           sig.transaction.nonce != first_tx.nonce ||
           sig.message_hash != *message_hash {
            anyhow::bail!("Partial signatures are for different transactions");
        }
    }
    
    if partial_signatures.len() < share.threshold {
        anyhow::bail!(
            "Not enough partial signatures. Need {} but got {}", 
            share.threshold, 
            partial_signatures.len()
        );
    }
    
    // Parse signature recids
    let mut sig_map = BTreeMap::new();
    for sig in partial_signatures {
        let recid: SignatureRecid = serde_json::from_slice(&sig.signature_recid)?;
        sig_map.insert(sig.share_index, recid);
    }
    
    // Replace the following with the correct function calls based on the library's documentation
    // let combined_sig = multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::combine_signatures(
    //     share.threshold,
    //     &sig_map,
    //     &share.public_key,
    // )?;
    // Adjust this part according to the available functions in the library
    
    // Extract r, s, v components for Ethereum transaction
    let r = combined_sig.r.to_bytes()?;
    let s = combined_sig.s.to_bytes()?;
    let v = combined_sig.recid as u64 + 27;
    
    // Adjust v for EIP-155
    let chain_id = share.transaction.chain_id;
    let v_eip155 = v + chain_id * 2 + 8;
    
    // Reconstruct the transaction
    let tx = TransactionRequest::new()
        .to(first_tx.to.clone())
        .value(parse_ether(first_tx.value.clone())?)
        .nonce(first_tx.nonce)
        .gas_price(U256::from_dec_str(&format!("{:.0}", 
            f64::from_str(&first_tx.gas_price)? * 1e9))?)
        .gas(first_tx.gas_limit)
        .chain_id(chain_id);
    
    // Create a signed transaction (RLP encoded)
    let rlp_tx = tx.rlp_signed(
        &r.into(),
        &s.into(),
        v_eip155,
    );
    
    // Create output
    let combined = CombinedSignature {
        transaction: first_tx.clone(),
        r: hex::encode(r),
        s: hex::encode(s),
        v: v_eip155,
        signed_tx: hex::encode(rlp_tx),
    };
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Save the combined signature
    let json = serde_json::to_string_pretty(&combined)?;
    fs::write(output_path, json)?;
    
    println!("Successfully combined {} partial signatures", partial_signatures.len());
    println!("To: {}, Value: {} ETH", first_tx.to, first_tx.value);
    println!("Combined signature saved to: {}", output_path.display());
    
    Ok(())
}