use crate::errors::{Result, WalletError};
use ethers::{
    prelude::*,
    types::{transaction::eip2718::TypedTransaction, TransactionRequest, U256, H256},
    utils::keccak256,
};
use std::str::FromStr;

#[derive(Debug)]
pub struct EthereumClient {
    provider: Provider<Http>,
    chain_id: u64,
}

impl EthereumClient {
    pub async fn new(rpc_url: &str, chain_id: u64) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| WalletError::EthersProvider(format!("Failed to connect to Ethereum node: {}", e)))?;
        
        Ok(Self {
            provider,
            chain_id,
        })
    }
    
    pub async fn get_balance(&self, address: &str) -> Result<U256> {
        let address = Address::from_str(address)
            .map_err(|e| WalletError::Ethereum(format!("Invalid address: {}", e)))?;
        
        self.provider.get_balance(address, None)
            .await
            .map_err(|e| WalletError::Ethereum(format!("Failed to get balance: {}", e)))
    }
    
    pub async fn get_nonce(&self, address: &str) -> Result<U256> {
        let address = Address::from_str(address)
            .map_err(|e| WalletError::Ethereum(format!("Invalid address: {}", e)))?;
        
        self.provider.get_transaction_count(address, None)
            .await
            .map_err(|e| WalletError::Ethereum(format!("Failed to get nonce: {}", e)))
    }
    
    pub fn build_transaction(
        &self,
        from: &str,
        to: &str,
        value: U256,
        nonce: U256,
        gas_price: Option<U256>,
        gas_limit: Option<U256>,
        data: Option<Vec<u8>>,
    ) -> Result<(TypedTransaction, [u8; 32])> {
        let from_addr = Address::from_str(from)
            .map_err(|e| WalletError::Ethereum(format!("Invalid from address: {}", e)))?;
        
        let to_addr = Address::from_str(to)
            .map_err(|e| WalletError::Ethereum(format!("Invalid to address: {}", e)))?;
        
        let mut tx = TransactionRequest::new()
            .from(from_addr)
            .to(to_addr)
            .value(value)
            .nonce(nonce)
            .chain_id(self.chain_id);
        
        if let Some(gas_price) = gas_price {
            tx = tx.gas_price(gas_price);
        }
        
        if let Some(gas_limit) = gas_limit {
            tx = tx.gas(gas_limit);
        }
        
        if let Some(data) = data {
            tx = tx.data(data);
        }
        
        let typed_tx: TypedTransaction = tx.into();
        
        let hash = typed_tx.sighash().to_fixed_bytes();
        
        Ok((typed_tx, hash))
    }
    
    pub async fn send_transaction(&self, tx: TypedTransaction, signature: Vec<u8>) -> Result<H256> {
        // Extract the r, s, v components from the signature
        if signature.len() != 65 {
            return Err(WalletError::Ethereum("Invalid signature length".to_string()));
        }
        
        let r = U256::from_big_endian(&signature[0..32]);
        let s = U256::from_big_endian(&signature[32..64]);
        let v = signature[64] as u64 + self.chain_id * 2 + 35;
        
        // Apply the signature to the transaction
        let signed_tx = tx.rlp_signed(&Signature {
            r,
            s,
            v: v.into(),
        });
        
        // Send the raw transaction
        let pending_tx = self.provider.send_raw_transaction(signed_tx)
            .await
            .map_err(|e| WalletError::Ethereum(format!("Failed to send transaction: {}", e)))?;
            
        Ok(pending_tx.tx_hash())
    }
    
    pub fn derive_address_from_public_key(public_key_hex: &str) -> Result<String> {
        let public_key_bytes = hex::decode(public_key_hex)
            .map_err(|e| WalletError::Ethereum(format!("Invalid public key: {}", e)))?;
        
        let public_key = secp256k1::PublicKey::from_slice(&public_key_bytes)
            .map_err(|e| WalletError::Ethereum(format!("Invalid public key: {}", e)))?;
        
        // Convert to uncompressed form for Ethereum
        let uncompressed = public_key.serialize_uncompressed();
        
        // Take the keccak256 hash of the public key (excluding the first byte which is the format marker)
        let hash = keccak256(&uncompressed[1..]);
        
        // Take the last 20 bytes of the hash as the Ethereum address
        let address = format!("0x{}", hex::encode(&hash[12..]));
        
        Ok(address)
    }
} 