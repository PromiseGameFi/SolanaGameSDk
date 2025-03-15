use anyhow::Result;
use ethers::{
    core::types::{Address, Bytes, TransactionRequest, U256},
    providers::{Http, Middleware, Provider},
    signers::Signer,
};
use std::sync::Arc;
use std::str::FromStr;

mod key_management;
mod signer;
mod transaction;
mod ethereum;

use key_management::{KeyShare, TssKeyManager};
use signer::TssSigner;
use transaction::EthereumTransaction;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Creating MPC-TSS Ethereum Wallet on Sepolia");
    
    // Demo parameters
    let total_shares = 3;
    let threshold = 2;
    
    // Step 1: Generate key shares
    println!("\n\n1. Generating key shares ({} of {} scheme)...", threshold, total_shares);
    let (shares, public_key) = TssKeyManager::generate_shares(total_shares, threshold)?;
    
    // Print shares (in a real application, these would be distributed securely)
    for (i, share) in shares.iter().enumerate() {
        println!("Share {}: {}", i+1, share.to_hex());
    }
    
    // Step 2: Derive Ethereum address from public key
    let eth_address = ethereum::public_key_to_address(&public_key);
    println!("\n2. Derived Ethereum address: {}", eth_address);
    
    // Step 3: Create a transaction
    println!("\n3. Creating a sample transaction");
    let transaction = EthereumTransaction {
        nonce: U256::from(0u64),
        gas_price: U256::from(30_000_000_000u64), // 30 gwei
        gas_limit: U256::from(21_000u64),
        to: Some(Address::from_str("0x742d35Cc6634C0532925a3b844Bc454e4438f44e")?),
        value: U256::from(1_000_000_000_000_000u64), // 0.001 ETH
        data: Bytes::default(),
        chain_id: 11155111, // Sepolia chain ID
    };
    println!("{:?}", transaction);
    
    // Step 4: Sign transaction using threshold signature scheme
    println!("\n4. Signing transaction with threshold signature...");
    // In real world, shares would be distributed and signing would be interactive
    // For demo, we'll use the first 'threshold' number of shares
    let signing_shares = shares[..threshold].to_vec();
    let signature = TssSigner::sign_transaction(&transaction, &signing_shares)?;
    println!("Signature: 0x{}", hex::encode(&signature));
    
    // Step 5: Broadcast to Sepolia
    println!("\n5. Broadcasting transaction to Sepolia");
    // Replace with your Alchemy RPC endpoint
    let alchemy_url = "https://eth-sepolia.g.alchemy.com/v2/YOUR_ALCHEMY_API_KEY";
    let provider = Provider::<Http>::try_from(alchemy_url)?;
    
    let tx_request = transaction.to_transaction_request();
    let signed_tx = transaction.apply_signature(signature.clone())?;
    
    // Uncomment to actually broadcast
    // let pending_tx = provider.send_raw_transaction(signed_tx).await?;
    // println!("Transaction sent: {}", pending_tx.tx_hash());
    
    println!("Transaction hex: 0x{}", hex::encode(signed_tx));
    println!("\nNote: Transaction broadcasting is commented out. Replace Alchemy API key and uncomment to broadcast.");
    
    Ok(())
}