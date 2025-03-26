use structopt::StructOpt;
use std::path::PathBuf;
use curv::elliptic::curves::secp256_k1::GE;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::Keygen;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::sign::Sign;
use web3::types::{Address, TransactionParameters, U256};
use std::str::FromStr;

#[derive(StructOpt)]
enum Command {
    #[structopt(name = "generate-keys")]
    GenerateKeys {
        #[structopt(long)]
        threshold: u16,
        #[structopt(long)]
        shares: u16,
    },
    #[structopt(name = "sign")]
    Sign {
        #[structopt(long)]
        share_id: u16,
        #[structopt(long)]
        to: String,
        #[structopt(long)]
        amount: f64,
    },
    #[structopt(name = "combine")]
    Combine {
        #[structopt(long = "signature-files")]
        signature_files: Vec<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = Command::from_args();

    match cmd {
        Command::GenerateKeys { threshold, shares } => {
            generate_key_shares(threshold, shares).await?;
        }
        Command::Sign { share_id, to, amount } => {
            sign_transaction(share_id, &to, amount).await?;
        }
        Command::Combine { signature_files } => {
            combine_and_send_transaction(signature_files).await?;
        }
    }

    Ok(())
}

async fn generate_key_shares(threshold: u16, shares: u16) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize keygen parameters
    let params = keygen::Parameters {
        threshold: threshold as u16,
        share_count: shares as u16,
    };

    // Generate shares
    for i in 1..=shares {
        let share = Keygen::create_share(params.clone(), i)?;
        
        // Save share to file
        let share_file = format!("share_{}.json", i);
        std::fs::write(&share_file, serde_json::to_string(&share)?)?;
        println!("Generated share {} and saved to {}", i, share_file);
    }

    Ok(())
}

async fn sign_transaction(share_id: u16, to: &str, amount: f64) -> Result<(), Box<dyn std::error::Error>> {
    // Load share from file
    let share_file = format!("share_{}.json", share_id);
    let share: Share = serde_json::from_str(&std::fs::read_to_string(share_file)?)?;

    // Create transaction parameters
    let to_address = Address::from_str(to)?;
    let amount_wei = web3::types::U256::from((amount * 1e18) as u64);
    
    let tx_params = TransactionParameters {
        to: Some(to_address),
        value: amount_wei,
        ..Default::default()
    };

    // Create partial signature
    let partial_sig = Sign::create_partial_signature(&share, &tx_params)?;

    // Save partial signature
    let sig_file = format!("sig_{}_{}.json", share_id, hex::encode(&partial_sig.id));
    std::fs::write(&sig_file, serde_json::to_string(&partial_sig)?)?;
    
    println!("Created partial signature and saved to {}", sig_file);
    Ok(())
}

async fn combine_and_send_transaction(signature_files: Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    // Load partial signatures
    let mut partial_sigs = Vec::new();
    for file in signature_files {
        let sig: PartialSignature = serde_json::from_str(&std::fs::read_to_string(file)?)?;
        partial_sigs.push(sig);
    }

    // Combine signatures
    let final_signature = Sign::combine_signatures(&partial_sigs)?;

    // Connect to Sepolia network
    let transport = web3::transports::Http::new("https://sepolia.infura.io/v3/713582309bac484c98256ff6a93e9bac")?;
    let web3 = web3::Web3::new(transport);

    // Send transaction
    let tx_hash = web3.eth().send_raw_transaction(final_signature.into()).await?;
    println!("Transaction sent! Hash: {:?}", tx_hash);

    Ok(())
}
