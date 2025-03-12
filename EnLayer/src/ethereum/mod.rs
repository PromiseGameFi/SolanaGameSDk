use ethers::{
    providers::{Provider, Http},
    types::{Address, U256},
    middleware::Middleware,
};
use anyhow::Result;
use std::sync::Arc;

pub struct EthereumClient {
    provider: Arc<Provider<Http>>,
    chain_id: u64,
}

impl EthereumClient {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let chain_id = provider.get_chainid().await?.as_u64();
        Ok(Self {
            provider: Arc::new(provider),
            chain_id,
        })
    }

    pub async fn send_test_transaction(
        &self,
        _from: Address,
        _to: Address,
        _value: U256,
        signature: Vec<u8>,
    ) -> Result<()> {
        let raw_tx = ethers::types::Bytes::from(signature);
        let pending_tx = self.provider.send_raw_transaction(raw_tx).await?;

        if let Some(receipt) = pending_tx.await? {
            println!("Transaction sent: {:?}", receipt.transaction_hash);
        } else {
            println!("Transaction pending");
        }

        Ok(())
    }

    pub async fn get_balance(&self, address: Address) -> Result<U256> {
        Ok(self.provider.get_balance(address, None).await?)
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub async fn get_gas_price(&self) -> Result<U256> {
        Ok(self.provider.get_gas_price().await?)
    }

    pub async fn get_transaction_count(&self, address: Address) -> Result<U256> {
        Ok(self.provider.get_transaction_count(address, None).await?)
    }
}