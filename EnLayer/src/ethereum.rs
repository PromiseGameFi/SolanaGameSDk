use ethers::{
    prelude::*,
    signers::{LocalWallet, Signer},
};

pub struct EthereumInterface {
    provider: Provider<Http>,
    wallet: LocalWallet,
}

impl EthereumInterface {
    pub async fn new(rpc_url: &str, private_key: &str) -> Result<Self, EthError> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let wallet = private_key.parse::<LocalWallet>()?;
        
        Ok(Self {
            provider,
            wallet,
        })
    }
    
    pub async fn send_transaction(
        &self,
        to: Address,
        value: U256,
        signature: Vec<u8>
    ) -> Result<TransactionReceipt, EthError> {
        let tx = TransactionRequest::new()
            .to(to)
            .value(value)
            .from(self.wallet.address());
            
        let signed_tx = self.wallet.sign_transaction(&tx).await?;
        let pending_tx = self.provider.send_transaction(signed_tx, None).await?;
        
        Ok(pending_tx.await?)
    }
}