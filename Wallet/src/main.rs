use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ethers::providers::{Http, Provider, Middleware};
use ethers::core::types::transaction::eip2718::TypedTransaction;
use ethers::core::types::{Bytes, TransactionRequest};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::Runtime;

mod key_splitter;
mod partial_signer;
mod signature_combiner;

use key_splitter::{generate_shares, load_share};
use partial_signer::create_partial_signature;
use signature_combiner::{combine_signatures, CombinedSignature};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate MPC-TSS key shares
    GenerateShares {
        /// Total number of shares to generate
        #[arg(short, long)]
        total: usize,

        /// Threshold number of shares required for signing
        #[arg(short, long)]
        threshold: usize,

        /// Directory to store key shares
        #[arg(short, long, default_value = "shares")]
        output_dir: PathBuf,
    },
    /// Create a partial signature using a key share
    PartialSign {
        /// Path to the key share file
        #[arg(short, long)]
        share_path: PathBuf,

        /// Recipient Ethereum address
        #[arg(short, long)]
        to: String,

        /// Amount to send in Ether
        #[arg(short, long)]
        amount: f64,

        /// Nonce for the transaction
        #[arg(short, long, default_value = "0")]
        nonce: u64,

        /// Gas price in Gwei
        #[arg(short, long, default_value = "5.0")]
        gas_price: f64,

        /// Gas limit
        #[arg(short, long, default_value = "21000")]
        gas_limit: u64,

        /// Path to output the partial signature
        #[arg(short, long)]
        output_path: PathBuf,
    },
    /// Combine partial signatures into a complete signature
    CombineSignatures {
        /// Paths to partial signature files
        #[arg(short, long)]
        signature_paths: Vec<PathBuf>,

        /// Path to any key share (for public key)
        #[arg(short, long)]
        share_path: PathBuf,

        /// Path to output the combined signature
        #[arg(short, long, default_value = "combined_signature.json")]
        output_path: PathBuf,
    },
    /// Send a transaction to Sepolia using a combined signature
    SendTransaction {
        /// Path to the combined signature file
        #[arg(short, long)]
        signature_path: PathBuf,

        /// Infura API key for Sepolia
        #[arg(short, long, env = "INFURA_API_KEY")]
        infura_api_key: String,
    },
}

async fn send_transaction(signature_path: &Path, infura_api_key: &str) -> Result<()> {
    // Load the combined signature
    let signature_data = std::fs::read_to_string(signature_path)
        .with_context(|| format!("Failed to read signature from {}", signature_path.display()))?;
    
    let combined: CombinedSignature = serde_json::from_str(&signature_data)
        .with_context(|| "Failed to parse combined signature")?;
    
    // Create a provider for Sepolia
    let rpc_url = format!("https://sepolia.infura.io/v3/{}", infura_api_key);
    let provider = Provider::<Http>::try_from(rpc_url)
        .with_context(|| "Failed to create Ethereum provider")?;
    
    let provider = Arc::new(provider);
    
    // Decode the signed transaction
    let signed_tx_bytes = hex::decode(&combined.signed_tx)
        .with_context(|| "Failed to decode signed transaction hex")?;
    
    // Send the raw transaction
    let tx_hash = provider
        .send_raw_transaction(Bytes::from(signed_tx_bytes))
        .await
        .with_context(|| "Failed to send transaction")?;
    
    println!("Transaction sent successfully!");
    println!("Transaction hash: {}", tx_hash);
    println!("View on Sepolia Etherscan: https://sepolia.etherscan.io/tx/{}", tx_hash);
    
    Ok(())
}

fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    match cli.command {
        Commands::GenerateShares {
            total,
            threshold,
            output_dir,
        } => {
            generate_shares(total, threshold, &output_dir)
        },
        Commands::PartialSign {
            share_path,
            to,
            amount,
            nonce,
            gas_price,
            gas_limit,
            output_path,
        } => {
            let share = load_share(&share_path)?;
            let rt = Runtime::new()?;
            rt.block_on(create_partial_signature(
                share, &to, amount, nonce, gas_price, gas_limit, &output_path
            ))
        },
        Commands::CombineSignatures {
            signature_paths,
            share_path,
            output_path,
        } => {
            let partial_signatures = signature_paths
                .iter()
                .map(|path| {
                    let data = std::fs::read_to_string(path)
                        .with_context(|| format!("Failed to read signature from {}", path.display()))?;
                    
                    let sig = serde_json::from_str(&data)
                        .with_context(|| format!("Failed to parse signature from {}", path.display()))?;
                    
                    Ok(sig)
                })
                .collect::<Result<Vec<_>>>()?;
            
            let share = load_share(&share_path)?;
            
            combine_signatures(partial_signatures, &output_path, &share)
        },
        Commands::SendTransaction {
            signature_path,
            infura_api_key,
        } => {
            let rt = Runtime::new()?;
            rt.block_on(send_transaction(&signature_path, &infura_api_key))
        },
    }
}