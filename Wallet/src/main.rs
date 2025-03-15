use clap::{Parser, Subcommand};
use ethers::{
    providers::{Middleware, Provider, Http},
    types::{TransactionRequest, U256, H160},
    utils::keccak256,
};
use secp256k1::Message;
use serde::{Serialize, Deserialize};
use std::convert::TryFrom;
use threshold_crypto::{SecretKeySet, SecretKeyShare, SignatureShare, PublicKeySet};
use rand::Rng;
use anyhow::Result;

#[derive(Serialize, Deserialize)]
struct SerializableSecretKeyShare {
    index: usize,
    bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct SerializableSignatureShare {
    bytes: Vec<u8>,
}

#[derive(Parser)]
#[command(name = "mpc_tss_wallet")]
#[command(about = "A CLI tool for MPC-TSS wallet operations", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate shares for each party
    GenerateShares {
        #[arg(short, long)]
        total_shares: usize,
        #[arg(short, long)]
        threshold: usize,
    },
    /// Generate partial signatures
    GeneratePartialSigs {
        #[arg(short, long)]
        shares: Vec<String>,
        #[arg(short, long)]
        message: String,
    },
    /// Reconstruct and verify the full signature
    ReconstructSignature {
        #[arg(short, long)]
        partial_sigs: Vec<String>,
        #[arg(short, long)]
        message: String,
    },
    /// Broadcast a transaction to Sepolia
    BroadcastTransaction {
        #[arg(short, long)]
        signature: String,
        #[arg(short, long)]
        recipient: String,
        #[arg(short, long)]
        value: u64,
        #[arg(short, long)]
        nonce: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateShares { total_shares, threshold } => 
            generate_shares(total_shares, threshold),
        Commands::GeneratePartialSigs { shares, message } => 
            generate_partial_signatures(shares, message),
        Commands::ReconstructSignature { partial_sigs, message } => 
            reconstruct_signature(partial_sigs, message),
        Commands::BroadcastTransaction { signature, recipient, value, nonce } => 
            broadcast_transaction(signature, recipient, value, nonce).await,
    }
}

fn generate_shares(total_shares: usize, threshold: usize) -> Result<()> {
    let mut rng = rand::thread_rng();
    let sk_set = SecretKeySet::random(threshold - 1, &mut rng);
    let pk_set = sk_set.public_keys();

    println!("Public Key: {}", hex::encode(pk_set.public_key().to_bytes()));

    for i in 0..total_shares {
        let share = sk_set.secret_key_share(i);
        let serialized = SerializableSecretKeyShare {
            index: i,
            bytes: share.serialize().to_vec(),
        };
        let json = serde_json::to_string(&serialized)?;
        println!("Share {}: {}", i, json);
    }

    Ok(())
}

fn generate_partial_signatures(shares: Vec<String>, message: String) -> Result<()> {
    let message_hash = keccak256(message.as_bytes());
    let message = Message::from_slice(&message_hash)?;

    for share_str in shares {
        let serialized: SerializableSecretKeyShare = serde_json::from_str(&share_str)?;
        let share = SecretKeyShare::deserialize(&serialized.bytes)?;
        let partial_sig = share.sign(&message.serialize());
        
        let serialized_sig = SerializableSignatureShare {
            bytes: partial_sig.serialize().to_vec(),
        };
        let json = serde_json::to_string(&serialized_sig)?;
        println!("Partial Signature: {}", json);
    }

    Ok(())
}

fn reconstruct_signature(partial_sigs: Vec<String>, message: String) -> Result<()> {
    let message_hash = keccak256(message.as_bytes());
    let message = Message::from_slice(&message_hash)?;
    let sk_set = SecretKeySet::random(1, &mut rand::thread_rng());
    let pk_set = sk_set.public_keys();

    let mut shares = Vec::new();
    for (i, sig_str) in partial_sigs.iter().enumerate() {
        let serialized: SerializableSignatureShare = serde_json::from_str(sig_str)?;
        let sig = SignatureShare::deserialize(&serialized.bytes)?;
        shares.push((i, sig));
    }

    let full_sig = pk_set.combine_signatures(&shares)?;
    let is_valid = pk_set.public_key().verify(&full_sig, &message.serialize());

    println!("Full Signature: {}", hex::encode(full_sig.to_bytes()));
    println!("Signature Valid: {}", is_valid);

    Ok(())
}

async fn broadcast_transaction(signature: String, recipient: String, value: u64, nonce: u64) -> Result<()> {
    let provider = Provider::<Http>::try_from(
        "https://eth-sepolia.g.alchemy.com/v2/YOUR_ALCHEMY_API_KEY",
    )?;

    let recipient: H160 = recipient.parse()?;
    let tx = TransactionRequest::new()
        .to(recipient)
        .value(U256::from(value))
        .gas(U256::from(21000))
        .gas_price(U256::from(1000000000))
        .nonce(U256::from(nonce))
        .chain_id(11155111);

    let pending = provider.send_transaction(tx, None).await?;
    println!("Transaction broadcasted: {:?}", pending.tx_hash());

    Ok(())
}