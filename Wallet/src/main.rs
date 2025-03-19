use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

mod wallet;
mod crypto;
mod ethereum;
mod error;

use wallet::{KeyShare, ShareManager};
use error::WalletError;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate key shares for the MPC-TSS wallet
    GenerateShares {
        /// Total number of shares to create
        #[arg(short, long)]
        total: usize,
        
        /// Threshold number of shares required for signing
        #[arg(short, long)]
        threshold: usize,
        
        /// Output directory to save the shares
        #[arg(short, long, default_value = "shares")]
        output_dir: PathBuf,
    },
    
    /// Generate a partial signature using a key share
    SignTransaction {
        /// Path to the key share file
        #[arg(short, long)]
        share_path: PathBuf,
        
        /// Recipient address
        #[arg(short, long)]
        to: String,
        
        /// Amount in ETH to send
        #[arg(short, long)]
        amount: f64,
        
        /// Output file for the partial signature
        #[arg(short, long, default_value = "partial_sig.json")]
        output: PathBuf,
    },
    
    /// Combine partial signatures to create a full signature
    CombineSignatures {
        /// Paths to partial signature files
        #[arg(short, long)]
        partial_signatures: Vec<PathBuf>,
        
        /// Output file for the full signature
        #[arg(short, long, default_value = "full_sig.json")]
        output: PathBuf,
    },
    
    /// Send a signed transaction to the Sepolia network
    SendTransaction {
        /// Path to the full signature file
        #[arg(short, long)]
        signature_path: PathBuf,
        
        /// RPC URL for Sepolia (optional, uses default if not provided)
        #[arg(short, long)]
        rpc_url: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), WalletError> {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::GenerateShares { total, threshold, output_dir } => {
            println!("{}", "Generating key shares...".green());
            let share_manager = ShareManager::new(*total, *threshold)?;
            share_manager.generate_and_save_shares(output_dir)?;
            println!("{} {} {}", 
                     "Successfully generated".green(),
                     total.to_string().yellow(),
                     "key shares!".green());
            println!("{} {} {}", 
                     "Threshold for signing:".cyan(),
                     threshold.to_string().yellow(),
                     "shares".cyan());
            println!("{} {}", 
                     "Shares saved to:".cyan(),
                     output_dir.display().to_string().yellow());
            Ok(())
        },
        
        Commands::SignTransaction { share_path, to, amount, output } => {
            println!("{}", "Generating partial signature...".green());
            let share = KeyShare::load_from_file(share_path)?;
            let partial_sig = share.sign_transaction(to, *amount)?;
            partial_sig.save_to_file(output)?;
            println!("{} {}", 
                     "Partial signature saved to:".green(),
                     output.display().to_string().yellow());
            Ok(())
        },
        
        Commands::CombineSignatures { partial_signatures, output } => {
            println!("{}", "Combining signatures...".green());
            let result = wallet::combine_signatures(partial_signatures)?;
            result.save_to_file(output)?;
            println!("{} {}", 
                     "Full signature saved to:".green(),
                     output.display().to_string().yellow());
            Ok(())
        },
        
        Commands::SendTransaction { signature_path, rpc_url } => {
            println!("{}", "Sending transaction to Sepolia network...".green());
            let tx_hash = ethereum::send_transaction(signature_path, rpc_url.clone()).await?;
            println!("{}", "Transaction sent successfully!".green());
            println!("{} {}", 
                     "Transaction hash:".cyan(),
                     tx_hash.yellow());
            
            let network = if let Some(url) = rpc_url {
                url
            } else {
                "Sepolia"
            };
            println!("{} {}", 
                     "Network:".cyan(),
                     network.yellow());
            Ok(())
        },
    }
} 