use dotenv::dotenv;
use std::env;
use mpc_tss_wallet::{MPCWallet, ethereum::EthereumClient};
use ethers::{
    types::{Address, U256},
    utils::rlp,
};
use std::str::FromStr;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use serde::{Serialize, Deserialize};

// Wrapper struct for serialization
#[derive(Serialize, Deserialize)]
struct ShareWrapper {
    index: u32,
    value: Vec<u8>,
}

// Conversion implementations for ShareWrapper to/from the internal Share type
impl From<ShareWrapper> for mpc_tss_wallet::crypto::Share {
    fn from(wrapper: ShareWrapper) -> Self {
        Self {
            index: wrapper.index,
            value: wrapper.value,
        }
    }
}

impl From<mpc_tss_wallet::crypto::Share> for ShareWrapper {
    fn from(share: mpc_tss_wallet::crypto::Share) -> Self {
        Self {
            index: share.index,
            value: share.value,
        }
    }
}

// Struct to represent a private key file format
#[derive(Serialize, Deserialize)]
struct KeyFile {
    private_key: String,
}

// Struct to represent a share file format, including address information
#[derive(Serialize, Deserialize)]
struct ShareFile {
    index: u32,
    value: Vec<u8>,
    address: String,
}

// CLI parsing structure using clap
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// Subcommands for the CLI application
#[derive(Subcommand)]
enum Commands {
    // Split command: Creates key shares from a private key
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
    // Sign command: Creates a partial signature with one share
    Sign {
        #[arg(short = 'p', long)]
        share_path: String,
        #[arg(short = 't', long)]
        to: String,
        #[arg(short = 'v', long)]
        value: f64,
        #[arg(short = 'i', long)]
        share_index: u32,
    },
    // SendTx command: Combines signatures and sends the transaction
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
            // Read the private key from file
            let key_file_content = fs::read_to_string(key_file_path)?;
            let key_file: KeyFile = serde_json::from_str(&key_file_content)?;
            let private_key = hex::decode(key_file.private_key.trim_start_matches("0x"))?;
            
            // Create the MPC wallet with the specified threshold and shares
            let wallet = MPCWallet::new(&private_key, *threshold, *total_shares)?;
            let wallet_address = wallet.get_address();
            println!("Wallet address: 0x{}", hex::encode(wallet_address));

            // Create output directory and save each share to a file
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
        Commands::Sign {
            share_path,
            to,
            value,
            share_index,
        } => {
            // Initialize Ethereum client from environment variables
            let rpc_url = env::var("ETHEREUM_RPC_URL").expect("ETHEREUM_RPC_URL must be set");
            let client = EthereumClient::new(&rpc_url).await?;
            let to_address = Address::from_str(to)?;
            let value_wei = U256::from((value * 1e18) as u64);

            // Load the share from file
            let share_file: ShareFile = serde_json::from_str(&fs::read_to_string(share_path)?)?;
            let share = mpc_tss_wallet::crypto::Share {
                index: *share_index,
                value: share_file.value,
            };

            // Get transaction parameters from the network
            let nonce = client.get_transaction_count(Address::from_str(&share_file.address)?).await?;
            let chain_id = client.chain_id();

            // Generate partial signature
            // Note: Using dummy key for wallet initialization since we're only using it for signing
            let dummy_key = [1u8; 32]; // Temporary key for wallet initialization
            let wallet = MPCWallet::new(&dummy_key, 2, 2)?; // Adjust threshold and total shares as needed
            let partial_sig = wallet.partial_sign_transaction(&share, to_address, value_wei, nonce, chain_id).await?;

            // Save the partial signature to a file
            let share_wrapper = ShareWrapper {
                index: *share_index,
                value: partial_sig, // Store partial signature
            };

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
            // Initialize Ethereum client and verify chain ID
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

            // Data structures to hold shares, signatures, and the sender address
            let mut shares = Vec::new();
            let mut partial_sigs = Vec::new();
            let mut sender_address = None;

            // Load shares and partial signatures from files
            for sig_path in signatures {
                println!("Reading signature file: {}", sig_path);
                let wrapper_content = fs::read_to_string(sig_path)?;
                let wrapper: ShareWrapper = serde_json::from_str(&wrapper_content)?;
                partial_sigs.push(wrapper.value.clone());

                // Determine share filename based on signature filename
                let share_filename = sig_path.replace("signature_", "share_");
                let share_path = format!("shares/{}", share_filename);
                println!("Reading share file: {}", share_path);
                let share_file_content = fs::read_to_string(&share_path)?;
                let share_file: ShareFile = serde_json::from_str(&share_file_content)?;

                // Validate address format
                if !share_file.address.starts_with("0x") || share_file.address.len() != 42 {
                    return Err(anyhow::anyhow!(
                        "Invalid address format in share file: {}",
                        share_file.address
                    ));
                }

                // Ensure all shares have the same sender address
                if let Some(ref addr) = sender_address {
                    if *addr != Address::from_str(&share_file.address)? {
                        return Err(anyhow::anyhow!(
                            "Inconsistent addresses in share files: {:?} vs {}",
                            addr,
                            share_file.address
                        ));
                    }
                } else {
                    sender_address = Some(Address::from_str(&share_file.address)?);
                }

                shares.push(mpc_tss_wallet::crypto::Share {
                    index: share_file.index,
                    value: share_file.value,
                });
            }

            // Extract sender address and prepare transaction parameters
            let sender_address = sender_address.ok_or_else(|| anyhow::anyhow!("No address found in share files"))?;
            let to_address = Address::from_str(to)?;
            let value_wei = U256::from((value * 1e18) as u64);

            // Initialize wallet with dummy key for signature combination
            let wallet = MPCWallet::new(&[1u8; 32], shares.len() as u32, shares.len() as u32)?;

            // Combine partial signatures to get the final signature
            let final_signature = wallet.combine_signatures(&partial_sigs).await?;
            if final_signature.len() != 65 {
                return Err(anyhow::anyhow!(
                    "Expected 65-byte signature, got {} bytes",
                    final_signature.len()
                ));
            }

            println!("Final combined signature: {}", hex::encode(&final_signature));

            // Extract signature components (r, s, v) from the combined signature
            let r = U256::from_big_endian(&final_signature[0..32]);
            let s = U256::from_big_endian(&final_signature[32..64]);
            let v_raw = final_signature[64] as u64;

            // Calculate correct v parameter according to EIP-155
            let recovery_id = if v_raw == 0 || v_raw == 1 {
                v_raw
            } else {
                return Err(anyhow::anyhow!("Unexpected v_raw value: {}. Expected 0 or 1.", v_raw));
            };
            let v = chain_id * 2 + 35 + recovery_id;

            // Fetch additional transaction parameters from the network
            let balance = client.get_balance(sender_address).await?;
            println!("Sender address: 0x{}", hex::encode(sender_address));
            println!("Sender balance: {} Wei ({} ETH)", balance, balance.as_u128() as f64 / 1e18);

            let gas_price = client.get_gas_price().await?;
            let gas_limit = U256::from(21000);
            let gas_cost = gas_price * gas_limit;
            let total_cost = value_wei + gas_cost;

            let nonce = client.get_transaction_count(sender_address).await?;
            println!("Using nonce: {}", nonce);

            // Construct RLP-encoded transaction according to EIP-155
            let tx_rlp = {
                let mut rlp = rlp::RlpStream::new_list(9);
                rlp.append(&nonce);
                rlp.append(&gas_price);
                rlp.append(&gas_limit);
                rlp.append(&to_address);
                rlp.append(&value_wei);
                rlp.append(&Vec::<u8>::new()); // data
                rlp.append(&U256::from(v));
                rlp.append(&r);
                rlp.append(&s);
                rlp.out().to_vec()
            };

            println!("RLP-encoded tx: {}", hex::encode(&tx_rlp));

            // Send transaction with retry mechanism (up to 3 attempts)
            let max_attempts = 3;
            for attempt in 1..=max_attempts {
                let latest_balance = client.get_balance(sender_address).await?;
                println!("Attempt {}: Latest balance: {} Wei", attempt, latest_balance);
                
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

                // Actually send the transaction
                match client.send_test_transaction(sender_address, to_address, value_wei, tx_rlp.clone()).await {
                    Ok(_) => {
                        println!("Transaction sent successfully!");
                        return Ok(());
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