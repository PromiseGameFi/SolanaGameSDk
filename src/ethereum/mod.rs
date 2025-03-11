use ethers::{
    providers::{Provider, Http},
    types::{Address, U256, TransactionRequest},
    middleware::Middleware,
};
use anyhow::Result;
use std::sync::Arc;

pub struct EthereumClient {
    provider: Arc<Provider<Http>>,
}

impl EthereumClient {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        Ok(Self {
            provider: Arc::new(provider),
        })
    }

    pub async fn send_test_transaction(
        &self,
        from: Address,
        to: Address,
        value: U256,
        signature: Vec<u8>,
    ) -> Result<()> {
        let tx = TransactionRequest::new()
            .to(to)
            .value(value)
            .from(from);

        let pending_tx = self.provider
            .send_raw_transaction(signature.into())
            .await?;

        let receipt = pending_tx.await?;
        println!("Transaction sent: {:?}", receipt.transaction_hash);

        Ok(())
    }

    pub async fn get_balance(&self, address: Address) -> Result<U256> {
        Ok(self.provider.get_balance(address, None).await?)
    }
} 