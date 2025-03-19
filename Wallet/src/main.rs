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
use std::io::Write;

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
    /// Get Ethereum address from a key share
    GetAddress {
        /// Share ID to use
        #[clap(short, long)]
        share_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::GenerateKeys { threshold, shares } => {
            println!("Generating {} key shares with threshold {}", shares, threshold);
            println!("WARNING: Threshold and share count CANNOT be changed after generation!");
            
            // Prompt for confirmation
            print!("Do you want to continue? (y/n): ");
            std::io::stdout().flush().unwrap();
            
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Key generation cancelled");
                return Ok(());
            }
            
            let key_manager = KeyManager::new();
            let key_shares = key_manager.generate_key_shares(threshold, shares)?;
            
            // Save each share to a separate file and derive the Ethereum address
            let mut address = None;
            for (id, share) in key_shares {
                key_manager.save_key_share(&id, &share)?;
                println!("Saved key share {} to share_{}.json", id, id);
                
                // Derive address from the first share (all shares have the same public key)
                if address.is_none() {
                    address = Some(EthereumClient::derive_address_from_public_key(&share.verifying_key)?);
                }
            }
            
            println!("Key generation completed successfully.");
            println!("Wallet address: {}", address.unwrap());
            println!("\nIMPORTANT: This wallet has a FIXED threshold of {} out of {} shares.", 
                     threshold, shares);
            println!("If you need different security parameters, generate a new wallet and transfer funds.");
            
            Ok(())
        },
        Commands::Sign { share_id, to, amount } => {
            println!("Creating partial signature with share {}", share_id);
            
            let key_manager = KeyManager::new();
            let key_share = key_manager.load_key_share(&share_id)?;
            
            let signer = Signer::new();
            
            let rpc_url = env::var("ETH_RPC_URL")
                .map_err(|_| WalletError::Ethereum("ETH_RPC_URL not set".to_string()))?;
            
            let chain_id = env::var("ETH_CHAIN_ID")
                .map_err(|_| WalletError::Ethereum("ETH_CHAIN_ID not set".to_string()))?
                .parse::<u64>()
                .map_err(|_| WalletError::Ethereum("Invalid ETH_CHAIN_ID".to_string()))?;
            
            let eth_client = EthereumClient::new(&rpc_url, chain_id).await?;
            
            // Derive Ethereum address from public key
            let address = EthereumClient::derive_address_from_public_key(&key_share.verifying_key)?;
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
            
            // Create partial signature WITHOUT reconstructing the private key
            let partial_sig = signer.create_partial_signature(&key_share, &tx_hash)?;
            
            // Save signature data
            let signature_data = serde_json::json!({
                "share_id": partial_sig.share_id,
                "commitment": partial_sig.commitment,
                "response": partial_sig.response,
                "threshold": partial_sig.threshold,
                "total_shares": partial_sig.total_shares,
                "tx_hash": hex::encode(tx_hash),
                "tx_data": serde_json::to_string(&tx)?,
                "verifying_key": key_share.verifying_key,
            });
            
            let signature_file = format!("sig_{}_{}.json", share_id, hex::encode(&tx_hash[0..4]));
            fs::write(&signature_file, serde_json::to_string_pretty(&signature_data)?)?;
            
            println!("Partial signature saved to {}", signature_file);
            
            Ok(())
        },
        Commands::Combine { signature_files } => {
            println!("Combining signatures from {} files", signature_files.len());
            
            // Load all partial signatures
            let mut tx_data = None;
            let mut tx_hash = None;
            let mut partial_sigs = Vec::new();
            let mut verifying_key = None;
            
            for file in &signature_files {
                let data = fs::read_to_string(file)?;
                let sig_data: serde_json::Value = serde_json::from_str(&data)?;
                
                let partial_sig = PartialSignature {
                    share_id: sig_data["share_id"].as_str().unwrap().to_string(),
                    commitment: sig_data["commitment"].as_str().unwrap().to_string(),
                    response: sig_data["response"].as_str().unwrap().to_string(),
                    threshold: sig_data["threshold"].as_u64().unwrap() as u16,
                    total_shares: sig_data["total_shares"].as_u64().unwrap() as u16,
                };
                
                // Keep track of transaction data and verifying key
                if tx_data.is_none() {
                    tx_data = Some(sig_data["tx_data"].as_str().unwrap().to_string());
                    tx_hash = Some(hex::decode(sig_data["tx_hash"].as_str().unwrap())?);
                    verifying_key = Some(sig_data["verifying_key"].as_str().unwrap().to_string());
                }
                
                partial_sigs.push(partial_sig);
            }
            
            if partial_sigs.is_empty() {
                return Err(WalletError::Threshold("No signature files provided".to_string()));
            }
            
            let tx_hash = tx_hash.unwrap();
            let verifying_key = verifying_key.unwrap();
            
            // Combine signatures WITHOUT reconstructing the private key
            let signer = Signer::new();
            let signature = signer.combine_signatures(
                &partial_sigs,
                &tx_hash,
                &verifying_key,
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
            
            Ok(())
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
            
            Ok(())
        },
        Commands::GetAddress { share_id } => {
            println!("Getting Ethereum address from share {}", share_id);
            
            let key_manager = KeyManager::new();
            let key_share = key_manager.load_key_share(&share_id)?;
            
            let address = EthereumClient::derive_address_from_public_key(&key_share.verifying_key)?;
            
            println!("Ethereum Address: {}", address);
            println!("Threshold: {} of {} shares", key_share.threshold, key_share.total_shares);
            
            Ok(())
        },
    }
}
