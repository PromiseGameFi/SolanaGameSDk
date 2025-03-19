use anyhow::{Context, Result};
use curv::arithmetic::Converter;
use curv::elliptic::curves::{Point, Scalar, Secp256k1};
use ethers::core::types::{Address, TransactionRequest, U256};
use ethers::utils::rlp::Rlp;
use ethers::utils::{hash_message, keccak256, parse_ether};
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::{
    SignatureRecid,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::str::FromStr;

use crate::key_splitter::KeyShare;

#[derive(Serialize, Deserialize, Clone)]
pub struct TransactionData {
    pub to: String,
    pub value: String, // In Ether
    pub nonce: u64,
    pub gas_price: String, // In Gwei
    pub gas_limit: u64,
    pub data: String,
    pub chain_id: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PartialSignature {
    pub share_index: usize,
    pub transaction: TransactionData,
    pub message_hash: String,
    pub signature_recid: Vec<u8>, // Serialized SignatureRecid
}

pub async fn create_partial_signature(
    share: KeyShare,
    to_address: &str,
    value_eth: f64,
    nonce: u64,
    gas_price_gwei: f64,
    gas_limit: u64,
    output_path: &Path,
) -> Result<()> {
    // Parse transaction data
    let to = Address::from_str(to_address)
        .context("Invalid recipient address")?;
    
    let value = parse_ether(value_eth.to_string())
        .context("Invalid ETH amount")?;
    
    let gas_price = U256::from((gas_price_gwei * 1e9) as u64);
    
    // Create a transaction request
    let tx = TransactionRequest::new()
        .to(to)
        .value(value)
        .nonce(nonce)
        .gas_price(gas_price)
        .gas(gas_limit)
        .chain_id(11155111); // Sepolia chain ID
    
    // Serialize and hash the transaction
    let tx_bytes = tx.rlp_unsigned();
    let message_hash = keccak256(&tx_bytes);
    
    // Initialize a secure random number generator
    let mut rng = OsRng;
    
    // Parse VSS scheme
    let vss_scheme_vec: Vec<Vec<u8>> = share.vss_scheme_vec;
    
    // Generate partial signature
    let signature_recid = multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::sign(
        &message_hash,
        &share.keys,
        &share.index,
        &share.threshold,
        &mut rng,
    )?;
    
    // Serialize the signature
    let signature_recid_bytes = serde_json::to_vec(&signature_recid)?;
    
    // Store transaction data and partial signature
    let tx_data = TransactionData {
        to: to_address.to_string(),
        value: value_eth.to_string(),
        nonce,
        gas_price: gas_price_gwei.to_string(),
        gas_limit,
        data: "0x".to_string(), // No contract data
        chain_id: 11155111,
    };
    
    let partial_sig = PartialSignature {
        share_index: share.index,
        transaction: tx_data,
        message_hash: hex::encode(message_hash),
        signature_recid: signature_recid_bytes,
    };
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Save the partial signature
    let json = serde_json::to_string_pretty(&partial_sig)?;
    fs::write(output_path, json)?;
    
    println!("Created partial signature with share {} for transaction", share.index);
    println!("To: {}, Value: {} ETH", to_address, value_eth);
    println!("Partial signature saved to: {}", output_path.display());
    
    Ok(())
}