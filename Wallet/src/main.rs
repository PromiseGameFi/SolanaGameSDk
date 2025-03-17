use clap::{Parser, Subcommand};
use mpc_tss_wallet::wallet::{self, *};
use std::path::PathBuf;
use std::fs;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    GenerateShares {
        #[arg(short, long)]
        threshold: usize,
        #[arg(short, long)]
        shares: usize,
        #[arg(short, long)]
        output_dir: PathBuf,
    },
    PartialSign {
        #[arg(short, long)]
        share_file: PathBuf,
        #[arg(short, long)]
        message: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    CombineSignatures {
        #[arg(short, long)]
        public_file: PathBuf,
        #[arg(short, long)]
        signature_files: Vec<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
    },
    SendTransaction {
        #[arg(short, long)]
        rpc_url: String,
        #[arg(short, long)]
        public_file: PathBuf,
        #[arg(short, long)]
        signature_file: PathBuf,
        #[arg(short, long)]
        to: String,
        #[arg(short, long)]
        value: f64,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateShares { threshold, shares, output_dir } => {
            fs::create_dir_all(&output_dir)?;
            let shares = wallet::keygen::generate_shares(threshold, shares)?;
            
            for (id, share) in shares {
                let path = output_dir.join(format!("share_{}.json", id));
                fs::write(path, serde_json::to_string(&share)?)?;
            }
        }
        Commands::PartialSign { share_file, message, output } => {
            let share: KeyShare = serde_json::from_str(&fs::read_to_string(share_file)?)?;
            let message = hex::decode(message.trim_start_matches("0x"))?;
            let signature = wallet::sign::partial_sign(&share, &message)?;
            fs::write(output, hex::encode(signature))?;
        }
        Commands::CombineSignatures { public_file, signature_files, output } => {
            let public: PublicKeyPackage = serde_json::from_str(&fs::read_to_string(public_file)?)?;
            let message = vec![]; // Should be passed from context
            
            let mut signatures = vec![];
            for file in signature_files {
                let sig = hex::decode(fs::read_to_string(file)?)?;
                let id = file.file_stem().unwrap().to_str().unwrap().split('_').last().unwrap().parse()?;
                signatures.push((id, sig));
            }
            
            let combined = wallet::sign::combine_signatures(&public.public_package, &signatures, &message)?;
            fs::write(output, hex::encode(combined.serialize_der()))?;
        }
        Commands::SendTransaction { rpc_url, public_file, signature_file, to, value } => {
            let public: PublicKeyPackage = serde_json::from_str(&fs::read_to_string(public_file)?)?;
            let signature = hex::decode(fs::read_to_string(signature_file)?)?;
            
            let provider = Provider::<Http>::try_from(&rpc_url)?;
            let chain_id = provider.get_chainid().await?.as_u64();
            let nonce = provider.get_transaction_count(derive_address(&public), None).await?;
            
            let (tx, _) = wallet::network::construct_transaction(
                to.parse()?,
                U256::from((value * 1e18) as u128),
                nonce,
                chain_id
            );
            
            let tx_hash = wallet::network::send_transaction(
                &rpc_url,
                &public,
                &signature,
                tx
            ).await?;
            
            println!("Transaction sent: 0x{}", hex::encode(tx_hash));
        }
    }

    Ok(())
}
