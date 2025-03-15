use mpc_tss_wallet::MpcTssWallet;
use ethers::prelude::*;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize wallet with 5 shares, threshold 3
    let wallet = MpcTssWallet::new(5, 3)?;
    let address = wallet.address();
    println!("Wallet Address: {:?}", address);

    // Create a sample transaction (send 0.01 ETH)
    let tx = TransactionRequest::new()
        .to("0x Recipient Address Here ".parse::<H160>()?)
        .value(U256::from(10_000_000_000_000_000u64)) // 0.01 ETH in wei
        .gas(U256::from(21_000))
        .gas_price(U256::from(20_000_000_000u64)) // 20 gwei
        .nonce(U256::zero())
        .chain_id(11155111); // Sepolia chain ID

    // Sign with 3 shares (indices 0, 1, 2)
    let signature = wallet.sign_transaction(tx.clone(), vec![0, 1, 2]).await?;
    println!("Signature: {:?}", signature);

    // Broadcast to Sepolia
    let tx_hash = broadcast_to_sepolia(tx, signature).await?;
    println!("Transaction Hash: {:?}", tx_hash);

    Ok(())
}