mod errors;
mod key_manager;
mod signer;
mod eth;

use errors::{Result, WalletError};
use key_manager::{KeyManager, KeyShare};
use signer::Signer;
use eth::EthereumClient;

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use ethers::types::U256;
use frost_secp256k1::{Identifier, SigningCommitment, SigningResponse, SigningPackage, Parameters};
use rand::rngs::OsRng;
use secp256k1::Message;
use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use tokio;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate key shares for MPC
    GenerateKeys {
        /// Threshold (minimum number of shares needed)
        #[clap(short, long)]
        threshold: u16,
        /// Total number of shares to create
        #[clap(short, long)]
        shares: u16,
    },
    /// Create a partial signature for a transaction
    Sign {
        /// Share ID to use
        #[clap(short, long)]
        share_id: String,
        /// Recipient address
        #[clap(short, long)]
        to: String,
        /// Amount to send in ETH
        #[clap(short, long)]
        amount: String,
    },
    /// Combine partial signatures
    Combine {
        /// Path to the signature file
        #[clap(short, long)]
        signature_files: Vec<String>,
    },
    /// Get balance of the wallet
    Balance {
        /// Address to check
        #[clap(short, long)]
        address: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::GenerateKeys { threshold, shares } => {
            println!("Generating {} key shares with threshold {}", shares, threshold);
            
            let key_manager = KeyManager::new("wallet_config.json");
            let key_shares = key_manager.generate_key_shares(threshold, shares)?;
            
            for (id, share) in key_shares {
                key_manager.save_key_share(&id, &share)?;
                println!("Saved key share {} to share_{}.json", id, id);
            }
            
            println!("Key generation completed successfully.");
        },
        Commands::Sign { share_id, to, amount } => {
            println!("Creating partial signature with share {}", share_id);
            
            let key_manager = KeyManager::new("wallet_config.json");
            let config = key_manager.load_config()?;
            let key_share = key_manager.load_key_share(&share_id)?;
            
            let signer = Signer::new(config.threshold, config.shares);
            
            let rpc_url = env::var("ETH_RPC_URL")
                .map_err(|_| WalletError::Ethereum("ETH_RPC_URL not set".to_string()))?;
            
            let chain_id = env::var("ETH_CHAIN_ID")
                .map_err(|_| WalletError::Ethereum("ETH_CHAIN_ID not set".to_string()))?
                .parse::<u64>()
                .map_err(|_| WalletError::Ethereum("Invalid ETH_CHAIN_ID".to_string()))?;
            
            let eth_client = EthereumClient::new(&rpc_url, chain_id).await?;
            
            // Deserialize key package to get the public key
            let key_package: frost_secp256k1::KeyPackage = serde_json::from_str(&key_share.key_package)?;
            let verifying_key = key_package.verifying_key();
            
            // Derive Ethereum address from public key
            let public_key_bytes = verifying_key.serialize();
            let address = format!("0x{}", hex::encode(&ethers::utils::keccak256(&public_key_bytes)[12..]));
            
            println!("Wallet address: {}", address);
            
            let nonce = eth_client.get_nonce(&address).await?;
            
            // Convert ETH amount to wei
            let amount_wei = ethers::utils::parse_ether(amount)
                .map_err(|e| WalletError::Ethereum(format!("Invalid amount: {}", e)))?;
            
            // Build transaction
            let (tx, tx_hash) = eth_client.build_transaction(
                &address,
                &to,
                amount_wei,
                nonce,
                None,
                None,
                None,
            )?;
            
            println!("Transaction hash to sign: {}", hex::encode(tx_hash));
            
            // Create partial signature
            let (id, commitment, signature_share) = signer.create_partial_signature(&key_share, &tx_hash)?;
            
            // Save signature share
            let signature_data = serde_json::json!({
                "share_id": id,
                "commitment": serde_json::to_string(&commitment)?,
                "signature_share": serde_json::to_string(&signature_share)?,
                "tx_hash": hex::encode(tx_hash),
                "tx_data": serde_json::to_string(&tx)?,
            });
            
            let signature_file = format!("sig_{}_{}.json", id, hex::encode(&tx_hash[0..4]));
            std::fs::write(&signature_file, serde_json::to_string_pretty(&signature_data)?)?;
            
            println!("Partial signature saved to {}", signature_file);
        },
        Commands::Combine { signature_files } => {
            println!("Combining signatures from {} files", signature_files.len());
            
            let key_manager = KeyManager::new("wallet_config.json");
            let config = key_manager.load_config()?;
            
            if signature_files.len() < config.threshold as usize {
                return Err(WalletError::Threshold(format!(
                    "Need at least {} signature shares, but only {} provided",
                    config.threshold,
                    signature_files.len()
                )));
            }
            
            // Load all signature shares
            let mut tx_data = None;
            let mut tx_hash = None;
            let mut commitments = HashMap::new();
            let mut signature_shares = HashMap::new();
            
            for file in &signature_files {
                let data = std::fs::read_to_string(file)?;
                let sig_data: serde_json::Value = serde_json::from_str(&data)?;
                
                let share_id = sig_data["share_id"].as_str().unwrap();
                let commitment: SigningCommitment = serde_json::from_str(
                    sig_data["commitment"].as_str().unwrap()
                )?;
                let signature_share: SigningResponse = serde_json::from_str(
                    sig_data["signature_share"].as_str().unwrap()
                )?;
                
                // Keep track of transaction data
                if tx_data.is_none() {
                    tx_data = Some(sig_data["tx_data"].as_str().unwrap().to_string());
                    tx_hash = Some(hex::decode(sig_data["tx_hash"].as_str().unwrap())?);
                }
                
                let identifier = Identifier::try_from(share_id.parse::<u16>().unwrap()).unwrap();
                commitments.insert(identifier, commitment);
                signature_shares.insert(identifier, signature_share);
            }
            
            let tx_hash = tx_hash.unwrap();
            let message = Message::from_slice(&tx_hash.as_slice()).unwrap();
            
            // Create signing package
            let signing_package = SigningPackage::new(commitments, message);
            
            // Combine signatures
            let signer = Signer::new(config.threshold, config.shares);
            let signature = signer.combine_signatures(signing_package, signature_shares)?;
            
            // Convert FROST signature to Ethereum signature format
            // This is a simplified implementation
            let r = signature.R.serialize();
            let s = signature.z.to_bytes();
            
            // In a real implementation, you would need proper conversion
            // This is just a placeholder
            let mut sig_bytes = Vec::with_capacity(65);
            sig_bytes.extend_from_slice(&r[..32]);
            sig_bytes.extend_from_slice(&s[..32]);
            sig_bytes.push(0); // recovery id (placeholder)
            
            println!("Combined signature: {}", hex::encode(&sig_bytes));
            
            // Send transaction
            let rpc_url = env::var("ETH_RPC_URL")
                .map_err(|_| WalletError::Ethereum("ETH_RPC_URL not set".to_string()))?;
            
            let chain_id = env::var("ETH_CHAIN_ID")
                .map_err(|_| WalletError::Ethereum("ETH_CHAIN_ID not set".to_string()))?
                .parse::<u64>()
                .map_err(|_| WalletError::Ethereum("Invalid ETH_CHAIN_ID".to_string()))?;
            
            let eth_client = EthereumClient::new(&rpc_url, chain_id).await?;
            
            // Recreate transaction (simplified)
            let tx: ethers::types::transaction::eip2718::TypedTransaction = 
                serde_json::from_str(&tx_data.unwrap())?;
            
            // Serialize and send transaction (this is a simplified implementation)
            // In a real system, you would need to properly encode the transaction with the signature
            let tx_hash = eth_client.send_raw_transaction(vec![]).await?;
            
            println!("Transaction sent! Hash: {}", tx_hash);
        },
        Commands::Balance { address } => {
            let rpc_url = env::var("ETH_RPC_URL")
                .map_err(|_| WalletError::Ethereum("ETH_RPC_URL not set".to_string()))?;
            
            let chain_id = env::var("ETH_CHAIN_ID")
                .map_err(|_| WalletError::Ethereum("ETH_CHAIN_ID not set".to_string()))?
                .parse::<u64>()
                .map_err(|_| WalletError::Ethereum("Invalid ETH_CHAIN_ID".to_string()))?;
            
            let eth_client = EthereumClient::new(&rpc_url, chain_id).await?;
            
            let balance = eth_client.get_balance(&address).await?;
            let balance_eth = ethers::utils::format_ether(balance);
            
            println!("Balance of {}: {} ETH", address, balance_eth);
        },
    }
    
    Ok(())
}
