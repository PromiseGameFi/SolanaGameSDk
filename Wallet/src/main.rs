mod errors;
mod key_manager;
mod signer;
mod eth;

use errors::{Result, WalletError};
use key_manager::KeyManager;
use signer::{Signer, PartialSignature, NonceCommitment};
use key_manager::EcPoint;
use eth::EthereumClient;
use ethers::types::{U256, H256, Address, NameOrAddress};
use std::str::FromStr;

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
    /// Test RPC connection
    TestConnection {},
    /// Check transaction status
    CheckTransaction {
        /// Transaction hash to check
        #[clap(short, long)]
        hash: String,
    },
}

fn format_opt<T: std::fmt::Display>(opt: Option<T>) -> String {
    match opt {
        Some(val) => format!("{}", val),
        None => "0".to_string(),
    }
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
                Some(ethers::utils::parse_units("30", "gwei").unwrap().into()), // 30 Gwei gas price
                Some(U256::from(21_000u64)), // 21,000 gas limit
                None,
            )?;
            
            // Check the chain ID in the transaction
            if let Some(chain_id) = tx.chain_id() {
                println!("Transaction has chain_id: {}", chain_id);
                if chain_id.as_u64() != 11155111 {
                    println!("⚠️ WARNING: Transaction chain_id ({}) doesn't match Sepolia (11155111)",
                        chain_id);
                }
            } else {
                println!("❌ ERROR: Transaction doesn't have chain_id set when generating hash for signing!");
                return Err(WalletError::Ethereum("Transaction missing chain ID".to_string()));
            }
            
            println!("Transaction hash to sign: 0x{}", hex::encode(&tx_hash));
            println!("Full transaction hash (for reference): 0x{}", hex::encode(&tx_hash));
            
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
                "tx_params": {
                    "from": format!("{:?}", tx.from().unwrap_or(&Address::zero())),
                    "to": format!("{:?}", tx.to().unwrap_or(&NameOrAddress::Address(Address::zero()))),
                    "value": format_opt(tx.value().map(|v| v.to_string())),
                    "nonce": format_opt(tx.nonce().map(|n| n.to_string())),
                    "gas_limit": format_opt(tx.gas().map(|g| g.to_string())),
                    "gas_price": format_opt(tx.gas_price().map(|gp| gp.to_string())),
                    "chain_id": tx.chain_id().map(|c| c.as_u64()).unwrap_or(chain_id),
                },
                "public_key": key_share.public_key,
                "chain_id": tx.chain_id().map(|c| c.as_u64()).unwrap_or(chain_id),
            });
            
            let commitment_file = format!("commit_{}_tx{}.json", 
                share_id, 
                hex::encode(&tx_hash[0..8])  // Use 8 bytes instead of 4 for better uniqueness
            );
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
            let mut tx_params = None;
            
            // Extract tx_params from the first commitment file before the loop
            if let Some(commit_file) = commitment_files.first() {
                let data = fs::read_to_string(commit_file)?;
                let json: serde_json::Value = serde_json::from_str(&data)?;
                tx_params = json.get("tx_params").cloned();
            }
            
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
            
            // First extract chain_id from the commitment data
            let chain_id = if let Some(commit_data) = commitment_files.first() {
                let data = fs::read_to_string(commit_data)?;
                let json: serde_json::Value = serde_json::from_str(&data)?;
                json.get("chain_id").and_then(|v| v.as_u64())
            } else {
                None
            };
            
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
                "chain_id": chain_id,
                "public_key": key_share.public_key,
                "tx_params": tx_params,
            });
            
            let signature_file = format!("sig_{}_tx{}.json", 
                share_id, 
                hex::encode(&tx_hash[0..8])  // Same change here
            );
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
            
            // Extract the chain ID from the signature files
            let chain_id = if let Some(file) = signature_files.first() {
                let data = fs::read_to_string(file)?;
                let sig_json: serde_json::Value = serde_json::from_str(&data)?;
                sig_json.get("chain_id").and_then(|v| v.as_u64())
            } else {
                None
            };
            
            // First try to deserialize normally
            let mut tx: ethers::types::transaction::eip2718::TypedTransaction = 
                serde_json::from_str(&tx_data.unwrap())?;

            // Check if chain ID is missing
            if tx.chain_id().is_none() {
                println!("Transaction missing chain ID after deserialization - fixing from stored params");
                
                // Extract transaction parameters from first signature file
                let sig_data = fs::read_to_string(&signature_files[0])?;
                let sig_json: serde_json::Value = serde_json::from_str(&sig_data)?;
                
                // Get the chain ID and other params from tx_params
                if let Some(tx_params) = sig_json.get("tx_params") {
                    if let Some(chain_id) = tx_params.get("chain_id").and_then(|v| v.as_u64()) {
                        println!("Restoring chain ID from transaction parameters: {}", chain_id);
                        tx.set_chain_id(chain_id);
                    }
                } else if let Some(chain_id) = sig_json.get("chain_id").and_then(|v| v.as_u64()) {
                    println!("Restoring chain ID from signature data: {}", chain_id);
                    tx.set_chain_id(chain_id);
                }
            }
            
            // Now verify the from address
            if let Some(from) = tx.from() {
                let from_str = format!("{:?}", from);
                
                // Derive the address from the public key in the first signature
                if !partial_sigs.is_empty() {
                    let sig_data = fs::read_to_string(&signature_files[0])?;
                    let sig_json: serde_json::Value = serde_json::from_str(&sig_data)?;
                    if let Some(public_key) = sig_json.get("public_key").and_then(|v| v.as_str()) {
                        let derived_address = EthereumClient::derive_address_from_public_key(public_key)?;
                        
                        println!("Transaction From address: {}", from_str);
                        println!("Derived wallet address: {}", derived_address);
                        
                        if from_str.to_lowercase() != derived_address.to_lowercase() {
                            println!("⚠️ WARNING: Transaction From address doesn't match derived address!");
                            println!("This will cause your transaction to fail!");
                        }
                    }
                }
            }
            
            // Verify chain ID
            println!("Verifying chain ID...");
            
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
            
            println!("Using RPC URL: {}", rpc_url);
            
            let chain_id = env::var("ETH_CHAIN_ID")
                .map_err(|_| WalletError::Ethereum("ETH_CHAIN_ID not set".to_string()))?
                .parse::<u64>()
                .map_err(|_| WalletError::Ethereum("Invalid ETH_CHAIN_ID".to_string()))?;
            
            if chain_id != 11155111 {
                println!("⚠️ WARNING: Chain ID {} is not Sepolia (11155111)!", chain_id);
                println!("This will cause your transaction to fail or be sent to the wrong network!");
                println!("Update your .env file with ETH_CHAIN_ID=11155111");
                
                // Optionally force the correct chain ID
                // chain_id = 11155111;
                // println!("Forcing chain ID to 11155111 (Sepolia)");
            }
            
            let eth_client = EthereumClient::new(&rpc_url, chain_id).await?;
            
            // Test RPC connection before sending
            println!("Testing RPC connection before sending...");
            eth_client.check_connection().await?;
            
            // Send the transaction with the combined signature
            let tx_hash = eth_client.send_transaction(tx, signature).await?;
            
            let full_tx_hash_str = format!("0x{}", hex::encode(tx_hash.as_bytes()));
            println!("Transaction sent! Hash: {}", full_tx_hash_str);
            
            Ok(())
        },
        Commands::Balance { address } => {
            // Don't parse the address - just use the string directly
            let rpc_url = env::var("ETH_RPC_URL").map_err(|e| WalletError::Ethereum(format!("ETH_RPC_URL not set: {}", e)))?;
            let chain_id = env::var("ETH_CHAIN_ID").map_err(|e| WalletError::Ethereum(format!("ETH_CHAIN_ID not set: {}", e)))?.parse::<u64>().map_err(|e| WalletError::Ethereum(format!("Invalid ETH_CHAIN_ID: {}", e)))?;
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
        Commands::TestConnection {} => {
            println!("Testing RPC connection...");
            
            let rpc_url = env::var("ETH_RPC_URL")
                .map_err(|_| WalletError::Ethereum("ETH_RPC_URL not set".to_string()))?;
            
            let chain_id = env::var("ETH_CHAIN_ID")
                .map_err(|_| WalletError::Ethereum("ETH_CHAIN_ID not set".to_string()))?
                .parse::<u64>()
                .map_err(|_| WalletError::Ethereum("Invalid ETH_CHAIN_ID".to_string()))?;
            
            println!("Using RPC URL: {}", rpc_url);
            println!("Using Chain ID: {}", chain_id);
            
            let eth_client = EthereumClient::new(&rpc_url, chain_id).await?;
            eth_client.check_connection().await?;
            
            Ok(())
        },
        Commands::CheckTransaction { hash } => {
            let rpc_url = env::var("ETH_RPC_URL")
                .map_err(|_| WalletError::Ethereum("ETH_RPC_URL not set".to_string()))?;
            
            let chain_id = env::var("ETH_CHAIN_ID")
                .map_err(|_| WalletError::Ethereum("ETH_CHAIN_ID not set".to_string()))?
                .parse::<u64>()
                .map_err(|_| WalletError::Ethereum("Invalid ETH_CHAIN_ID".to_string()))?;
            
            let eth_client = EthereumClient::new(&rpc_url, chain_id).await?;
            
            // Parse the hash string, ensuring it has 0x prefix
            let hash_str = if !hash.starts_with("0x") {
                format!("0x{}", hash)
            } else {
                hash
            };
            
            // Convert to H256
            let tx_hash = H256::from_str(&hash_str)
                .map_err(|e| WalletError::Ethereum(format!("Invalid transaction hash: {}", e)))?;
            
            eth_client.check_transaction_status(tx_hash).await?;
            
            Ok(())
        },
    }
}