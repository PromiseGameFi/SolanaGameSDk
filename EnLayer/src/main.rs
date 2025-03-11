use eth_mpc_threshold::{MPCProtocol, Party, Communication, EthereumInterface};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize party
    let party = Party::new(1, threshold, total_parties);
    
    // Setup communication
    let comm = Communication::new(
        "127.0.0.1:8000".parse()?,
        setup_peers()
    ).await?;
    
    // Create MPC protocol instance
    let protocol = MPCProtocol::new(party, comm).await;
    
    // Connect to Sepolia
    let eth = EthereumInterface::new(
        "https://sepolia.infura.io/v3/YOUR-PROJECT-ID",
        "your-private-key"
    ).await?;
    
    // Generate signature
    let message = b"Transaction data...";
    let signature = protocol.generate_signature(message).await?;
    
    // Send transaction
    let receipt = eth.send_transaction(
        "0x...".parse()?,
        ethers::utils::parse_ether("0.1")?,
        signature
    ).await?;
    
    println!("Transaction sent: {:?}", receipt);
    Ok(())
}
