use eth_mpc_threshold::ethereum::transaction::EthereumTransaction;
use ethers::types::{Address, U256};
use anyhow::Result;
use std::fs;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    // Load configuration
    let rpc_url = std::env::var("ETHEREUM_RPC_URL")?;
    let to_address = Address::from_str("0x742d35Cc6634C0532925a3b844Bc454e4438f44e")?;
    let value = ethers::utils::parse_ether(0.1)?;

    // Load signature
    let signature = fs::read("signature.bin")?;

    // Send transaction
    let eth = EthereumTransaction::new(&rpc_url).await?;
    let receipt = eth.send_transaction(to_address, value, signature).await?;

    println!("Transaction sent! Hash: {:?}", receipt.transaction_hash);
    Ok(())
}