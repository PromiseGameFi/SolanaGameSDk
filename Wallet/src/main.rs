mod errors;
mod key_manager;
mod signer;
mod eth;

use errors::{Result, WalletError};
use key_manager::KeyManager;
use signer::{Signer, PartialSignature};
use eth::EthereumClient;

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use std::env;
use std::fs;
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
        threshold: u8,
        /// Total number of shares to create
        #[clap(short, long)]
        shares: u8,
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
            
            // Load the config to get the public key and derive the Ethereum address
            let _config = key_manager.load_config()?;
            let address = EthereumClient::derive_address_from_public_key(&_config.public_key)?;
            
            println!("Key generation completed successfully.");
            println!("Wallet address: {}", address);
        },
        Commands::Sign { share_id, to, amount } => {
            println!("Creating partial signature with share {}", share_id);
            
            let key_manager = KeyManager::new("wallet_config.json");
            let _config = key_manager.load_config()?;
            let key_share = key_manager.load_key_share(&share_id)?;
            
            let signer = Signer::new("wallet_config.json");
            
            let rpc_url = env::var("ETH_RPC_URL")
                .map_err(|_| WalletError::Ethereum("ETH_RPC_URL not set".to_string()))?;
            
            let chain_id = env::var("ETH_CHAIN_ID")
                .map_err(|_| WalletError::Ethereum("ETH_CHAIN_ID not set".to_string()))?
                .parse::<u64>()
                .map_err(|_| WalletError::Ethereum("Invalid ETH_CHAIN_ID".to_string()))?;
            
            let eth_client = EthereumClient::new(&rpc_url, chain_id).await?;
            
            // Derive Ethereum address from public key
            let address = EthereumClient::derive_address_from_public_key(&key_share.public_key)?;
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
            let partial_sig = signer.create_partial_signature(&key_share, &tx_hash)?;
            
            // Save signature data
            let signature_data = serde_json::json!({
                "share_id": partial_sig.share_id,
                "signature": partial_sig.signature,
                "tx_hash": hex::encode(tx_hash),
                "tx_data": serde_json::to_string(&tx)?,
            });
            
            let signature_file = format!("sig_{}_{}.json", share_id, hex::encode(&tx_hash[0..4]));
            fs::write(&signature_file, serde_json::to_string_pretty(&signature_data)?)?;
            
            println!("Partial signature saved to {}", signature_file);
        },
        Commands::Combine { signature_files } => {
            println!("Combining signatures from {} files", signature_files.len());
            
            let key_manager = KeyManager::new("wallet_config.json");
            let _config = key_manager.load_config()?;
            
            if signature_files.len() < _config.threshold as usize {
                return Err(WalletError::Threshold(format!(
                    "Need at least {} signature shares, but only {} provided",
                    _config.threshold,
                    signature_files.len()
                )));
            }
            
            // Load all partial signatures
            let mut tx_data = None;
            let mut tx_hash = None;
            let mut partial_sigs = Vec::new();
            
            for file in &signature_files {
                let data = fs::read_to_string(file)?;
                let sig_data: serde_json::Value = serde_json::from_str(&data)?;
                
                let partial_sig = PartialSignature {
                    share_id: sig_data["share_id"].as_str().unwrap().to_string(),
                    signature: sig_data["signature"].as_str().unwrap().to_string(),
                };
                
                // Keep track of transaction data
                if tx_data.is_none() {
                    tx_data = Some(sig_data["tx_data"].as_str().unwrap().to_string());
                    tx_hash = Some(hex::decode(sig_data["tx_hash"].as_str().unwrap())?);
                }
                
                partial_sigs.push(partial_sig);
            }
            
            let tx_hash = tx_hash.unwrap();
            
            // Combine signatures to get a complete signature
            let signer = Signer::new("wallet_config.json");
            let signature = signer.combine_signatures(
                partial_sigs,
                &tx_hash,
                _config.threshold,
            )?;
            
            println!("Combined signature: {}", hex::encode(&signature));
            
            // Send transaction
            let rpc_url = env::var("ETH_RPC_URL")
                .map_err(|_| WalletError::Ethereum("ETH_RPC_URL not set".to_string()))?;
            
            let chain_id = env::var("ETH_CHAIN_ID")
                .map_err(|_| WalletError::Ethereum("ETH_CHAIN_ID not set".to_string()))?
                .parse::<u64>()
                .map_err(|_| WalletError::Ethereum("Invalid ETH_CHAIN_ID".to_string()))?;
            
            let eth_client = EthereumClient::new(&rpc_url, chain_id).await?;
            
            // Parse transaction data
            let tx: ethers::types::transaction::eip2718::TypedTransaction = 
                serde_json::from_str(&tx_data.unwrap())?;
            
            // Send the transaction with the combined signature
            let tx_hash = eth_client.send_transaction(tx, signature).await?;
            
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
