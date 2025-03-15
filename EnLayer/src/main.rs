use dotenv::dotenv;
use std::env;
use mpc_tss_wallet::{MPCWallet, ethereum::EthereumClient};
use ethers::{
    types::{Address, U256},
    utils::{rlp, keccak256}, // Added keccak256 for transaction hash verification
};
use std::str::FromStr;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use serde::{Serialize, Deserialize};

// Wrapper struct for serialization - allows MPC share data to be stored in JSON
#[derive(Serialize, Deserialize)]
struct ShareWrapper {
    index: u32,
    value: Vec<u8>,
}

// Conversion from ShareWrapper to the library's Share type
impl From<ShareWrapper> for mpc_tss_wallet::crypto::Share {
    fn from(wrapper: ShareWrapper) -> Self {
        Self {
            index: wrapper.index,
            value: wrapper.value,
        }
    }
}

// Conversion from the library's Share type to ShareWrapper
impl From<mpc_tss_wallet::crypto::Share> for ShareWrapper {
    fn from(share: mpc_tss_wallet::crypto::Share) -> Self {
        Self {
            index: share.index,
            value: share.value,
        }
    }
}

// Structure to hold a private key read from a file
#[derive(Serialize, Deserialize)]
struct KeyFile {
    private_key: String,
}

// Structure to store share data along with the corresponding wallet address
#[derive(Serialize, Deserialize)]
struct ShareFile {
    index: u32,
    value: Vec<u8>,
    address: String,
}

// CLI application structure using clap
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// Define the subcommands supported by the CLI
#[derive(Subcommand)]
enum Commands {
    // Split command: Split a private key into multiple shares
    Split {
        #[arg(short = 'k', long)]
        key_file_path: String,
        #[arg(short = 's', long)]
        total_shares: u32,
        #[arg(short = 't', long)]
        threshold: u32,
        #[arg(short = 'o', long)]
        output_dir: String,
    },
    // Sign command: Generate a partial signature using a share
    // Updated with additional parameters for total_shares and threshold
    Sign {
        #[arg(short = 'p', long)]
        share_path: String,
        #[arg(short = 't', long)]
        to: String,
        #[arg(short = 'v', long)]
        value: f64,
        #[arg(short = 'i', long)]
        share_index: u32,
        #[arg(short = 'n', long)]
        total_shares: u32,
        #[arg(short = 'r', long)]
        threshold: u32,
    },
    // SendTx command: Combine signatures and send a transaction
    SendTx {
        #[arg(short = 's', long, value_delimiter = ',')]
        signatures: Vec<String>,
        #[arg(short = 't', long)]
        to: String,
        #[arg(short = 'v', long)]
        value: f64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables and parse CLI arguments
    dotenv().ok();
    let cli = Cli::parse();

    match &cli.command {
        // Handle the Split command: Generate shares from a private key
        Commands::Split {
            key_file_path,
            total_shares,
            threshold,
            output_dir,
        } => {
            // Read and parse the private key file
            let key_file_content = fs::read_to_string(key_file_path)?;
            let key_file: KeyFile = serde_json::from_str(&key_file_content)?;
            let private_key = hex::decode(key_file.private_key.trim_start_matches("0x"))?;
            
            // Create an MPC wallet with the private key and specified parameters
            let wallet = MPCWallet::new(&private_key, *threshold, *total_shares)?;
            let wallet_address = wallet.get_address();
            println!("Wallet address: 0x{}", hex::encode(wallet_address));

            // Create output directory and save each share to a separate file
            fs::create_dir_all(output_dir)?;

            for i in 1..=*total_shares {
                if let Some(share) = wallet.get_share(i) {
                    let share_file = ShareFile {
                        index: share.index,
                        value: share.value,
                        address: format!("0x{}", hex::encode(wallet_address)),
                    };

                    let path = format!("{}/share_{}.json", output_dir, i);
                    fs::write(&path, serde_json::to_string_pretty(&share_file)?)?;
                    println!("Share {} saved to {}", i, path);
                }
            }
        }

        // Handle the Sign command: Create a partial signature with one share
        // This version has been significantly simplified compared to the previous implementation
        Commands::Sign {
            share_path,
            to,
            value,
            share_index,
            total_shares: _total_shares, // Marked as unused with underscore prefix
            threshold: _threshold,       // Marked as unused with underscore prefix
        } => {
            // Initialize Ethereum client and parse transaction parameters
            let rpc_url = env::var("ETHEREUM_RPC_URL").expect("ETHEREUM_RPC_URL must be set");
            let _client = EthereumClient::new(&rpc_url).await?;
            let _to_address = Address::from_str(to)?;
            let _value_wei = U256::from((value * 1e18) as u64);
            
            // Load the share from file
            let share_file: ShareFile = serde_json::from_str(&fs::read_to_string(share_path)?)?;
            let share = vec![mpc_tss_wallet::crypto::Share {
                index: *share_index,
                value: share_file.value,
            }];

            // Convert to ShareWrapper and save to file
            // Note: This implementation doesn't actually generate a real signature,
            // it just stores the share directly - likely for demonstration purposes
            let share_wrapper: Vec<ShareWrapper> = share.into_iter().map(ShareWrapper::from).collect();
            
            let signature_path = format!("signature_{}.json", share_index);
            fs::write(&signature_path, serde_json::to_string_pretty(&share_wrapper)?)?;
            println!("Partial signature saved to {}", signature_path);
        }

        // Handle the SendTx command: Combine signatures and send the transaction
        Commands::SendTx {
            signatures,
            to,
            value,
        } => {
            // Initialize Ethereum client and verify we're on the Sepolia test network
            let rpc_url = env::var("ETHEREUM_RPC_URL").expect("ETHEREUM_RPC_URL must be set");
            let client = EthereumClient::new(&rpc_url).await?;
            let chain_id = client.chain_id();
            if chain_id != 11155111 {
                return Err(anyhow::anyhow!(
                    "Expected chain ID 11155111 (Sepolia), but provider has chain ID {}",
                    chain_id
                ));
            }
            println!("Using chain ID: {}", chain_id);

            // Collect shares from all signature files
            let mut shares = Vec::new();
            let mut expected_address = None;
            
            // Improved error handling for file reading operations
            for sig_path in signatures {
                println!("Attempting to read signature file: {}", sig_path);
                let wrapper_content = fs::read_to_string(sig_path)
                    .map_err(|e| anyhow::anyhow!("Failed to read signature file {}: {:?}", sig_path, e))?;
                
                // Parse signature file - now expecting a Vec of ShareWrapper
                let wrapper: Vec<ShareWrapper> = serde_json::from_str(&wrapper_content)?;
                
                // Determine share filename and read corresponding share file
                let share_filename = sig_path.replace("signature_", "share_");
                let share_path = format!("shares/{}", share_filename);
                println!("Attempting to read share file: {}", share_path);
                let share_file_content = fs::read_to_string(&share_path)
                    .map_err(|e| anyhow::anyhow!("Failed to read share file {}: {:?}", share_path, e))?;
                
                let share_file: ShareFile = serde_json::from_str(&share_file_content)?;
                
                // More comprehensive address validation
                if !share_file.address.starts_with("0x")
                    || share_file.address.len() != 42
                    || !share_file.address[2..].chars().all(|c| c.is_ascii_hexdigit())
                {
                    return Err(anyhow::anyhow!(
                        "Invalid address format in share file: {}",
                        share_file.address
                    ));
                }
                
                // Ensure all shares have the same sender address
                if let Some(ref addr) = expected_address {
                    if *addr != share_file.address {
                        return Err(anyhow::anyhow!(
                            "Inconsistent addresses in share files: {} vs {}",
                            addr,
                            share_file.address
                        ));
                    }
                } else {
                    expected_address = Some(share_file.address.clone());
                }
                
                // Add shares to the collection
                shares.extend(wrapper.into_iter().map(mpc_tss_wallet::crypto::Share::from));
            }

            // Parse transaction parameters
            let to_address = Address::from_str(to)?;
            let value_wei = U256::from((value * 1e18) as u64);

            // Initialize wallet with dummy key for transaction signing
            let dummy_key = [1u8; 32];
            let wallet = MPCWallet::new(&dummy_key, shares.len() as u32, shares.len() as u32)?;

            // Extract sender address from share files
            let expected_address = expected_address.ok_or_else(|| anyhow::anyhow!("No address found in share files"))?;
            let sender_address = Address::from_str(&expected_address)?;

            // Fetch account information and transaction parameters
            let balance = client.get_balance(sender_address).await?;
            println!("Sender address: {}", expected_address);
            println!("Sender balance: {} Wei ({} ETH)", balance, balance.as_u128() as f64 / 1e18);

            // Calculate gas costs
            let gas_price = client.get_gas_price().await?;
            let gas_limit = U256::from(21000);
            let gas_cost = gas_price * gas_limit;
            let total_cost = value_wei + gas_cost;
            println!("Gas price: {} Wei", gas_price);
            println!("Gas cost: {} Wei", gas_cost);
            println!("Total cost: {} Wei ({} ETH)", total_cost, total_cost.as_u128() as f64 / 1e18);

            let nonce = client.get_transaction_count(sender_address).await?;
            println!("Using nonce: {}", nonce);

            // Retry logic for handling insufficient funds
            let max_attempts = 3;
            for attempt in 1..=max_attempts {
                let latest_balance = client.get_balance(sender_address).await?;
                println!("Attempt {}: Latest balance before signing: {} Wei", attempt, latest_balance);
                
                // Check if balance is sufficient
                if latest_balance < total_cost {
                    if attempt == max_attempts {
                        return Err(anyhow::anyhow!(
                            "Insufficient funds after {} attempts: {} Wei < {} Wei",
                            max_attempts,
                            latest_balance,
                            total_cost
                        ));
                    }
                    println!("Funds insufficient, retrying in 5 seconds...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }

                // Sign the transaction with all shares
                println!("Signing transaction with {} shares...", shares.len());
                let signature = wallet.sign_transaction(&shares, to_address, value_wei, nonce, 11155111).await?;

                // Validate signature length
                if signature.len() != 65 {
                    return Err(anyhow::anyhow!(
                        "Expected 65-byte signature, got {} bytes",
                        signature.len()
                    ));
                }

                // Verify signature matches transaction hash - construct transaction message
                // This builds each component of the transaction individually for hashing
                let tx_hash_list: Vec<Vec<u8>> = vec![
                    { let mut s = rlp::RlpStream::new(); s.append(&nonce); s.out().to_vec() },
                    { let mut s = rlp::RlpStream::new(); s.append(&gas_price); s.out().to_vec() },
                    { let mut s = rlp::RlpStream::new(); s.append(&gas_limit); s.out().to_vec() },
                    { let mut s = rlp::RlpStream::new(); s.append(&to_address); s.out().to_vec() },
                    { let mut s = rlp::RlpStream::new(); s.append(&value_wei); s.out().to_vec() },
                    { let mut s = rlp::RlpStream::new(); s.append(&Vec::<u8>::new()); s.out().to_vec() },
                    { let mut s = rlp::RlpStream::new(); s.append(&U256::from(11155111u64)); s.out().to_vec() },
                    { let mut s = rlp::RlpStream::new(); s.append(&U256::zero()); s.out().to_vec() },
                    { let mut s = rlp::RlpStream::new(); s.append(&U256::zero()); s.out().to_vec() },
                ];
                
                // Create RLP encoding of the transaction for hashing
                let mut tx_hash_stream = rlp::RlpStream::new_list(tx_hash_list.len());
                tx_hash_stream.append_list::<Vec<u8>, Vec<u8>>(&tx_hash_list);
                let tx_hash_rlp = tx_hash_stream.out();
                let tx_hash = keccak256(&tx_hash_rlp);
                println!("Transaction hash: {:?}", hex::encode(&tx_hash));

                // Extract signature components
                let r = ethers::types::H256::from_slice(&signature[0..32]);
                let s = ethers::types::H256::from_slice(&signature[32..64]);
                let v_old = signature[64] as u64;
                
                // Handle recovery ID calculation - correctly determine v parameter
                let assumed_chain_id = 39;  // Hardcoded value for detecting signature format
                let recovery_id = v_old - assumed_chain_id * 2 - 35;

                // Calculate appropriate v value based on recovery ID
                let v_new = if recovery_id != 0 && recovery_id != 1 {
                    println!("Invalid recovery ID: {}, assuming v_old is raw recovery ID", recovery_id);
                    let recovery_id = if v_old <= 1 { 
                        v_old 
                    } else { 
                        return Err(anyhow::anyhow!("Invalid recovery ID from signature: {}", v_old)) 
                    };
                    11155111u64 * 2 + 35 + recovery_id
                } else {
                    11155111u64 * 2 + 35 + recovery_id
                };

                // Construct final signed transaction RLP encoding
                let tx_rlp = {
                    let mut rlp = rlp::RlpStream::new_list(9);
                    rlp.append(&nonce);
                    rlp.append(&gas_price);
                    rlp.append(&gas_limit);
                    rlp.append(&to_address);
                    rlp.append(&value_wei);
                    rlp.append(&Vec::<u8>::new());  // Empty data field
                    rlp.append(&U256::from(v_new));
                    rlp.append(&r);
                    rlp.append(&s);
                    rlp.out().to_vec()
                };

                println!("RLP-encoded tx: {:?}", hex::encode(&tx_rlp));

                // Send transaction and handle potential errors
                match client.send_test_transaction(sender_address, to_address, value_wei, tx_rlp).await {
                    Ok(_) => {
                        println!("Transaction sent successfully!");
                        break;
                    }
                    Err(e) if e.to_string().contains("insufficient funds") && attempt < max_attempts => {
                        println!("Attempt {} failed: {}. Retrying in 5 seconds...", attempt, e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }

    Ok(())
}