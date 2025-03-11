use dotenv::dotenv;
use std::env;
use mpc_tss_wallet::{MPCWallet, ethereum::EthereumClient};
use ethers::types::{Address, U256};
use std::str::FromStr;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Get RPC URL from environment
    let rpc_url = env::var("ETHEREUM_RPC_URL")
        .expect("ETHEREUM_RPC_URL must be set");

    // Initialize Ethereum client
    let client = EthereumClient::new(&rpc_url).await?;
    
    println!("Connected to Ethereum network");

    // Test parameters
    let threshold = 2;
    let total_shares = 3;
    
    // Create a test private key (DO NOT USE IN PRODUCTION)
    let private_key = [1u8; 32];

    // Create wallet
    let wallet = MPCWallet::new(&private_key, threshold, total_shares)?;
    println!("Wallet address: {:?}", wallet.get_address());

    // Collect shares (in practice, these would be distributed)
    let mut shares = Vec::new();
    for i in 1..=threshold {
        if let Some(share) = wallet.get_share(i) {
            shares.push(share);
        }
    }

    // Test transaction parameters
    let to = Address::from_str("0x742d35Cc6634C0532925a3b844Bc454e4438f44e")?;
    let value = U256::from(1000000000000000000u64); // 1 ETH
    let nonce = U256::zero();

    // Sign and send transaction
    let signature = wallet.sign_transaction(&shares, to, value, nonce).await?;
    
    client.send_test_transaction(
        wallet.get_address(),
        to,
        value,
        signature,
    ).await?;

    Ok(())
}