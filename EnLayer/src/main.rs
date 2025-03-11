use eth_mpc_threshold::{
    crypto::{
        shamir::{ShamirScheme, Share},
        signature::ThresholdSignature,
    },
    ethereum::transaction::EthereumTransaction,
};
use anyhow::Result;
use clap::{Parser, Subcommand};
use ethers::{
    types::{Address, U256},
    utils::parse_ether,
};
use std::{fs, str::FromStr};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate shares from a private key
    GenerateShares {
        #[arg(short, long)]
        threshold: u32,
        #[arg(short, long)]
        total_shares: u32,
        #[arg(short, long)]
        private_key_path: String,
    },
    /// Combine shares and create a signature
    CreateSignature {
        #[arg(short, long)]
        threshold: u32,
        #[arg(short, long)]
        message: String,
        #[arg(short, long, num_args = 1.., value_delimiter = ',')]
        share_paths: Vec<String>,
    },
    /// Send a transaction using the signature
    SendTransaction {
        #[arg(short, long)]
        to_address: String,
        #[arg(short, long)]
        amount_eth: f64,
        #[arg(short, long)]
        signature_path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Parse command line arguments
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateShares {
            threshold,
            total_shares,
            private_key_path,
        } => {
            println!("Generating shares...");
            generate_shares(threshold, total_shares, &private_key_path)?;
            println!("Shares generated successfully!");
        }
        Commands::CreateSignature {
            threshold,
            message,
            share_paths,
        } => {
            println!("Creating signature...");
            create_signature(threshold, &message, &share_paths)?;
            println!("Signature created successfully!");
        }
        Commands::SendTransaction {
            to_address,
            amount_eth,
            signature_path,
        } => {
            println!("Sending transaction...");
            send_transaction(&to_address, amount_eth, &signature_path).await?;
            println!("Transaction sent successfully!");
        }
    }

    Ok(())
}

fn generate_shares(threshold: u32, total_shares: u32, private_key_path: &str) -> Result<()> {
    // Read private key from file
    let private_key = fs::read(private_key_path)?;
    
    // Create Shamir scheme
    let scheme = ShamirScheme::new(threshold, total_shares);
    
    // Generate shares
    let shares = scheme.split_secret(&private_key)?;
    
    // Save shares to files
    for share in shares {
        let filename = format!("share_{}.json", share.index);
        let json = serde_json::to_string(&share)?;
        fs::write(filename, json)?;
        println!("Created share file: {}", filename);
    }
    
    Ok(())
}

fn create_signature(threshold: u32, message: &str, share_paths: &[String]) -> Result<()> {
    // Load shares from files
    let mut shares = Vec::new();
    for path in share_paths {
        let json = fs::read_to_string(path)?;
        let share: Share = serde_json::from_str(&json)?;
        shares.push(share);
    }
    
    // Verify we have enough shares
    if shares.len() < threshold as usize {
        anyhow::bail!("Not enough shares provided. Need at least {}", threshold);
    }
    
    // Create signature
    let threshold_sig = ThresholdSignature::new(shares, threshold);
    let signature = threshold_sig.sign_message(message.as_bytes())?;
    
    // Save signature
    fs::write("signature.bin", signature)?;
    println!("Signature saved to signature.bin");
    
    Ok(())
}

async fn send_transaction(
    to_address: &str,
    amount_eth: f64,
    signature_path: &str,
) -> Result<()> {
    // Get RPC URL from environment
    let rpc_url = std::env::var("ETHEREUM_RPC_URL")
        .expect("ETHEREUM_RPC_URL must be set");
    
    // Parse address and amount
    let to_address = Address::from_str(to_address)?;
    let amount = parse_ether(amount_eth)?;
    
    // Load signature
    let signature = fs::read(signature_path)?;
    
    // Create Ethereum transaction handler
    let eth = EthereumTransaction::new(&rpc_url).await?;
    
    // Send transaction
    let receipt = eth.send_transaction(to_address, amount, signature).await?;
    
    println!("Transaction hash: {:?}", receipt.transaction_hash);
    println!("Block number: {:?}", receipt.block_number);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_share_generation() -> Result<()> {
        // Test share generation
        let private_key = vec![1u8; 32];
        let temp_dir = tempfile::tempdir()?;
        let private_key_path = temp_dir.path().join("private_key");
        fs::write(&private_key_path, &private_key)?;

        generate_shares(3, 5, private_key_path.to_str().unwrap())?;

        // Verify shares were created
        for i in 1..=5 {
            let share_path = format!("share_{}.json", i);
            assert!(fs::metadata(&share_path).is_ok());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_signature_creation() -> Result<()> {
        // Test signature creation
        let share_paths = vec![
            "share_1.json".to_string(),
            "share_2.json".to_string(),
            "share_3.json".to_string(),
        ];
        
        create_signature(3, "test message", &share_paths)?;
        assert!(fs::metadata("signature.bin").is_ok());

        Ok(())
    }
}