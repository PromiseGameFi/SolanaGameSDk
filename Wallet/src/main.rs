mod errors;
mod key_manager;
mod signer;
mod eth;

use errors::{Result, WalletError};
use key_manager::KeyManager;
use signer::{Signer, PartialSignature, NonceCommitment};
use key_manager::EcPoint;
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
    /// Create nonce commitment (Round 1)
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
    /// Create partial signature (Round 2)
    SignRound2 {
        /// Share ID to use
        #[clap(short, long)]
        share_id: String,
        /// Commitment files from Round 1
        #[clap(short, long, num_args = 1..)]
        commitment_files: Vec<String>,
    },
    /// Combine partial signatures
    Combine {
        /// Path to the signature files
        #[clap(short, long, num_args = 1..)]
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
    /// Get address and balance from a key share
    GetWalletInfo {
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
                    address = Some(EthereumClient::derive_address_from_public_key(&share.public_key)?);
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
            println!("Creating signature with share {} - ROUND 1: Nonce commitment", share_id);
            
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
            
            println!("Transaction hash to sign: {}", hex::encode(&tx_hash));
            
            // ROUND 1: Create nonce commitment
            let commitment = signer.create_nonce_commitment(&key_share, &tx_hash)?;
            
            // Save commitment data
            let commitment_data = serde_json::json!({
                "share_id": commitment.share_id,
                "r_commitment": {
                    "x": commitment.r_commitment.x,
                    "y": commitment.r_commitment.y
                },
                "session_id": commitment.session_id,
                "tx_hash": hex::encode(&tx_hash),
                "tx_data": serde_json::to_string(&tx)?,
                "public_key": key_share.public_key,
            });
            
            let commitment_file = format!("commit_{}_{}.json", share_id, hex::encode(&tx_hash[0..4]));
            fs::write(&commitment_file, serde_json::to_string_pretty(&commitment_data)?)?;
            
            println!("Nonce commitment saved to {}", commitment_file);
            println!("Collect commitments from all signers, then run sign-round2 command");
            
            Ok(())
        },
        Commands::SignRound2 { share_id, commitment_files } => {
            println!("Creating signature with share {} - ROUND 2: Partial signature", share_id);
            
            let key_manager = KeyManager::new();
            let key_share = key_manager.load_key_share(&share_id)?;
            
            let signer = Signer::new();
            
            // Load all commitments
            let mut tx_hash = None;
            let mut tx_data = None;
            let mut commitments = Vec::new();
            let mut session_id = None;
            
            for file in &commitment_files {
                let data = fs::read_to_string(file)?;
                let commit_data: serde_json::Value = serde_json::from_str(&data)?;
                
                let commitment = NonceCommitment {
                    share_id: commit_data["share_id"].as_str().unwrap().to_string(),
                    r_commitment: EcPoint {
                        x: commit_data["r_commitment"]["x"].as_str().unwrap().to_string(),
                        y: commit_data["r_commitment"]["y"].as_str().unwrap().to_string(),
                    },
                    session_id: commit_data["session_id"].as_str().unwrap().to_string(),
                };
                
                if session_id.is_none() {
                    session_id = Some(commitment.session_id.clone());
                    tx_hash = Some(hex::decode(commit_data["tx_hash"].as_str().unwrap())?);
                    tx_data = Some(commit_data["tx_data"].as_str().unwrap().to_string());
                }
                
                commitments.push(commitment);
            }
            
            if commitments.is_empty() {
                return Err(WalletError::Signing("No commitments provided".to_string()));
            }
            
            let session_id = session_id.unwrap();
            let tx_hash = tx_hash.unwrap();
            
            // ROUND 2: Create partial signature
            let partial_sig = signer.create_partial_signature(
                &key_share, 
                &tx_hash, 
                &commitments, 
                &session_id
            )?;
            
            // Save signature data
            let signature_data = serde_json::json!({
                "share_id": partial_sig.share_id,
                "r_point": {
                    "x": partial_sig.r_point.x,
                    "y": partial_sig.r_point.y
                },
                "s_share": partial_sig.s_share,
                "threshold": partial_sig.threshold,
                "total_shares": partial_sig.total_shares,
                "session_id": partial_sig.session_id,
                "tx_hash": hex::encode(&tx_hash),
                "tx_data": tx_data,
            });
            
            let signature_file = format!("sig_{}_{}.json", share_id, hex::encode(&tx_hash[0..4]));
            fs::write(&signature_file, serde_json::to_string_pretty(&signature_data)?)?;
            
            println!("Partial signature saved to {}", signature_file);
            println!("Collect partial signatures from enough signers, then run combine command");
            
            Ok(())
        },
        Commands::Combine { signature_files } => {
            println!("Combining signatures from {} files", signature_files.len());
            
            // Load all partial signatures
            let mut tx_data = None;
            let mut tx_hash = None;
            let mut partial_sigs = Vec::new();
            
            for file in &signature_files {
                let data = fs::read_to_string(file)?;
                let sig_data: serde_json::Value = serde_json::from_str(&data)?;
                
                let partial_sig = PartialSignature {
                    share_id: sig_data["share_id"].as_str().unwrap().to_string(),
                    r_point: EcPoint {
                        x: sig_data["r_point"]["x"].as_str().unwrap().to_string(),
                        y: sig_data["r_point"]["y"].as_str().unwrap().to_string(),
                    },
                    s_share: sig_data["s_share"].as_str().unwrap().to_string(),
                    threshold: sig_data["threshold"].as_u64().unwrap() as u16,
                    total_shares: sig_data["total_shares"].as_u64().unwrap() as u16,
                    session_id: sig_data["session_id"].as_str().unwrap().to_string(),
                };
                
                // Keep track of transaction data
                if tx_data.is_none() {
                    tx_data = Some(sig_data["tx_data"].as_str().unwrap().to_string());
                    tx_hash = Some(hex::decode(sig_data["tx_hash"].as_str().unwrap())?);
                }
                
                partial_sigs.push(partial_sig);
            }
            
            if partial_sigs.is_empty() {
                return Err(WalletError::Threshold("No signature files provided".to_string()));
            }
            
            let tx_hash = tx_hash.unwrap();
            
            // Combine signatures
            let signer = Signer::new();
            let signature = signer.combine_signatures(
                &partial_sigs,
                &tx_hash,
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
            
            let address = EthereumClient::derive_address_from_public_key(&key_share.public_key)?;
            
            println!("Ethereum Address: {}", address);
            println!("Threshold: {} of {} shares", key_share.threshold, key_share.total_shares);
            
            Ok(())
        },
        Commands::GetWalletInfo { share_id } => {
            println!("Retrieving wallet info from share {}", share_id);
            
            // Load the key share
            let key_manager = KeyManager::new();
            let key_share = key_manager.load_key_share(&share_id)?;
            
            // Derive the Ethereum address from the public key
            let address = EthereumClient::derive_address_from_public_key(&key_share.public_key)?;
            println!("Ethereum Address: {}", address);
            println!("Threshold: {} of {} shares", key_share.threshold, key_share.total_shares);
            
            // Get the balance
            let rpc_url = env::var("ETH_RPC_URL")
                .map_err(|_| WalletError::Ethereum("ETH_RPC_URL not set".to_string()))?;
            
            let chain_id = env::var("ETH_CHAIN_ID")
                .map_err(|_| WalletError::Ethereum("ETH_CHAIN_ID not set".to_string()))?
                .parse::<u64>()
                .map_err(|_| WalletError::Ethereum("Invalid ETH_CHAIN_ID".to_string()))?;
            
            let eth_client = EthereumClient::new(&rpc_url, chain_id).await?;
            
            let balance = eth_client.get_balance(&address).await?;
            let balance_eth = ethers::utils::format_ether(balance);
            
            println!("Balance: {} ETH", balance_eth);
            
            Ok(())
        },
    }
}
