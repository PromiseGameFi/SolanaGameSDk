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
    Generate {
        #[arg(short, long)]
        threshold: usize,
        #[arg(short, long)]
        shares: usize,
        #[arg(short, long)]
        output_dir: PathBuf,
    },
    Sign {
        #[arg(short, long)]
        share: PathBuf,
        #[arg(short, long)]
        message: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    Combine {
        #[arg(short, long)]
        public: PathBuf,
        #[arg(short, long)]
        signatures: Vec<PathBuf>,
        #[arg(short, long)]
        message: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    Send {
        #[arg(short, long)]
        rpc_url: String,
        #[arg(short, long)]
        public: PathBuf,
        #[arg(short, long)]
        signature: PathBuf,
        #[arg(short, long)]
        to: String,
        #[arg(short, long)]
        value: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { threshold, shares, output_dir } => {
            fs::create_dir_all(&output_dir)?;
            wallet::keygen::generate_shares(threshold, shares, output_dir.to_str().unwrap())?;
        }
        
        Commands::Sign { share, message, output } => {
            let share: KeyShare = serde_json::from_str(&fs::read_to_string(share)?)?;
            let message = hex::decode(message.trim_start_matches("0x"))?;
            let sig = wallet::sign::partial_sign(&share, &message)?;
            fs::write(output, hex::encode(sig))?;
        }
        
        Commands::Combine { public, signatures, message, output } => {
            let public: PublicPackage = serde_json::from_str(&fs::read_to_string(public)?)?;
            let message = hex::decode(message.trim_start_matches("0x"))?;
            
            let mut sigs = vec![];
            for file in signatures {
                let id = file.file_stem().unwrap()
                    .to_str().unwrap()
                    .split('_').last().unwrap()
                    .parse::<usize>()?;
                let sig = hex::decode(fs::read_to_string(file)?)?;
                sigs.push((id, sig));
            }
            
            let combined = wallet::sign::combine_signatures(&public, &sigs, &message)?;
            fs::write(output, hex::encode(combined.serialize_der()))?;
        }
        
        Commands::Send { rpc_url, public, signature, to, value } => {
            let public: PublicPackage = serde_json::from_str(&fs::read_to_string(public)?)?;
            let signature = hex::decode(fs::read_to_string(signature)?)?;
            let value = U256::from_dec_str(&value)?;

            let (tx, _) = wallet::network::create_transaction(
                &rpc_url,
                &public,
                to.parse()?,
                value
            ).await?;

            let tx_hash = wallet::network::send_transaction(
                &rpc_url,
                tx,
                &signature
            ).await?;

            println!("Sent transaction: 0x{}", hex::encode(tx_hash));
        }
    }

    Ok(())
}
