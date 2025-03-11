use eth_mpc_threshold::crypto::{
    shamir::Share,
    signature::ThresholdSignature,
};
use anyhow::Result;
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    // Load shares
    let mut shares = Vec::new();
    for i in 1..=3 {
        let filename = format!("share_{}.json", i);
        let json = fs::read_to_string(filename)?;
        let share: Share = serde_json::from_str(&json)?;
        shares.push(share);
    }

    // Create signature
    let message = b"Hello, world!";
    let threshold_sig = ThresholdSignature::new(shares, 3);
    let signature = threshold_sig.sign_message(message)?;

    // Save signature
    fs::write("signature.bin", signature)?;
    println!("Signature created and saved!");
    Ok(())
}