use k256::ecdsa::{SigningKey, VerifyingKey};
use multi_party_ecdsa::protocols::threshold_ecdsa::{Keygen, Sign};
use rand::thread_rng;
use ethers::prelude::*;
use rlp::RlpStream;
use std::error::Error;

// MPC-TSS Wallet structure
pub struct MpcTssWallet {
    key_shares: Vec<Vec<u8>>, // n shares of the private key
    threshold: usize,         // k shares needed to reconstruct
    public_key: VerifyingKey, // Derived Ethereum public key
}

impl MpcTssWallet {
    /// Generate n key shares with threshold k
    pub fn new(n: usize, k: usize) -> Result<Self, Box<dyn Error>> {
        let mut rng = thread_rng();
        let keygen = Keygen::new(n, k, &mut rng)?;
        let shares: Vec<Vec<u8>> = keygen.generate()?;
        let public_key = keygen.derive_public_key()?;

        Ok(MpcTssWallet {
            key_shares: shares,
            threshold: k,
            public_key,
        })
    }

    /// Derive Ethereum address from the public key
    pub fn address(&self) -> H160 {
        let uncompressed = self.public_key.to_encoded_point(false);
        let public_key_bytes = &uncompressed.as_bytes()[1..]; // Remove 0x04 prefix
        let hash = ethers::utils::keccak256(public_key_bytes);
        H160::from_slice(&hash[12..])
    }

    /// Sign an Ethereum transaction with k partial signatures
    pub async fn sign_transaction(
        &self,
        tx: TransactionRequest,
        shares_to_use: Vec<usize>,
    ) -> Result<Bytes, Box<dyn Error>> {
        if shares_to_use.len() < self.threshold {
            return Err("Not enough shares to sign".into());
        }

        // Serialize transaction to RLP
        let mut rlp = RlpStream::new();
        tx.rlp(&mut rlp, None); // No signature yet
        let tx_bytes = rlp.out();
        let tx_hash = ethers::utils::keccak256(&tx_bytes);

        // Perform threshold signing
        let mut rng = thread_rng();
        let sign = Sign::new(&self.key_shares, &shares_to_use, &tx_hash, &mut rng)?;
        let partial_sigs = sign.partial_sign()?;
        let signature = sign.combine_signatures(&partial_sigs)?;

        // Convert to Ethereum signature format (r, s, v)
        let r = signature.r.to_bytes();
        let s = signature.s.to_bytes();
        let v = signature.v as u8 + 27; // Ethereum recovery ID adjustment

        let mut sig_bytes = Vec::new();
        sig_bytes.extend_from_slice(&r);
        sig_bytes.extend_from_slice(&s);
        sig_bytes.push(v);

        Ok(Bytes::from(sig_bytes))
    }
}

/// Broadcast a signed transaction to Sepolia
async fn broadcast_to_sepolia(
    tx: TransactionRequest,
    signature: Bytes,
) -> Result<H256, Box<dyn Error>> {
    let provider = Provider::<Http>::try_from(
        "https://eth-sepolia.g.alchemy.com/v2/YOUR_ALCHEMY_API_KEY",
    )?;
    let signed_tx = tx.rlp_signed(signature);
    let tx_hash = provider.send_raw_transaction(signed_tx).await?.tx_hash();
    Ok(tx_hash)
}