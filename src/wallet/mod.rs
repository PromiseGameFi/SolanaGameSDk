use crate::crypto::{KeySplitter, Share};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{Address, U256, TransactionRequest, TypedTransaction};
use ethers::middleware::SignerMiddleware;
use anyhow::Result;

pub struct MPCWallet {
    shares: Vec<Share>,
    threshold: u32,
    address: Address,
}

impl MPCWallet {
    pub fn new(private_key: &[u8], threshold: u32, total_shares: u32) -> Result<Self> {
        let splitter = KeySplitter::new(threshold, total_shares)?;
        let shares = splitter.split_key(private_key)?;
        
        // Generate Ethereum address from private key
        let wallet = LocalWallet::from_bytes(private_key)?;
        let address = wallet.address();

        Ok(Self {
            shares,
            threshold,
            address,
        })
    }

    pub fn get_address(&self) -> Address {
        self.address
    }

    pub fn get_share(&self, index: u32) -> Option<Share> {
        self.shares.iter()
            .find(|s| s.index == index)
            .cloned()
    }

    pub async fn sign_transaction(
        &self,
        shares: &[Share],
        to: Address,
        value: U256,
        nonce: U256,
    ) -> Result<Vec<u8>> {
        // Reconstruct private key from shares
        let private_key = KeySplitter::reconstruct_key(shares, self.threshold)?;
        
        // Create wallet from reconstructed key
        let wallet = LocalWallet::from_bytes(&private_key)?;
        
        // Create and sign transaction
        let tx = TransactionRequest::new()
            .to(to)
            .value(value)
            .nonce(nonce);
            
        let typed_tx: TypedTransaction = tx.into();
        let signature = wallet.sign_transaction(&typed_tx).await?;
        
        Ok(signature.to_vec())
    }
} 