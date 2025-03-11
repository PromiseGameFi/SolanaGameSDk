use eth_mpc_threshold::crypto::shamir::ShamirScheme;
use anyhow::Result;
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    // Load or generate private key
    let private_key = [1u8; 32]; // Replace with actual private key

    // Create shares
    let scheme = ShamirScheme::new(3, 5);
    let shares = scheme.split_secret(&private_key)?;

    // Save shares to files
    for share in shares {
        let filename = format!("share_{}.json", share.index);
        let json = serde_json::to_string(&share)?;
        fs::write(filename, json)?;
    }

    println!("Shares generated and saved!");
    Ok(())
}