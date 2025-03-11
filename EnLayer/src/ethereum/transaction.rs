use ethers::{
    prelude::*,
    types::{TransactionRequest, TransactionReceipt, Signature},
    providers::Middleware,
};
use anyhow::Result;

pub struct EthereumTransaction {
    provider: Provider<Http>,
    chain_id: u64,
}

impl EthereumTransaction {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let chain_id = provider.get_chainid().await?.as_u64();
        
        Ok(Self {
            provider,
            chain_id,
        })
    }

    pub async fn send_transaction(
        &self,
        tx: TransactionRequest,
        signature: Vec<u8>
    ) -> Result<TransactionReceipt> {
        let signature = Signature::try_from(signature.as_slice())?;
        let signed_tx = tx.rlp_signed(&signature);
        
        let pending_tx = self.provider.send_raw_transaction(signed_tx).await?;
        Ok(pending_tx.await?.expect("Transaction receipt not found"))
    }
}