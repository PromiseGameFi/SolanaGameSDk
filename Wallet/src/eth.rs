use crate::errors::{Result, WalletError};
use ethers::{
    prelude::*,
    types::{transaction::eip2718::TypedTransaction, TransactionRequest, U256},
    utils::keccak256,
};
use secp256k1::{PublicKey, SecretKey, Secp256k1, Message};
use std::str::FromStr;
use std::sync::Arc;

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
        
        // Calculate transaction hash (for signing)
        let encoded = typed_tx.rlp_unsigned().to_vec();
        let hash = keccak256(&encoded);
        
        Ok((typed_tx, hash))
    }
    
    pub async fn send_raw_transaction(&self, signed_tx: Vec<u8>) -> Result<H256> {
        self.provider.send_raw_transaction(Bytes::from(signed_tx))
            .await
            .map_err(|e| WalletError::Ethereum(format!("Failed to send transaction: {}", e)))
    }
    
    pub fn recover_public_key(signature: &[u8], message_hash: &[u8; 32]) -> Result<String> {
        // This is a simplified implementation
        // In a real scenario, you would need to convert FROST signature to Ethereum signature format
        
        let secp = Secp256k1::new();
        let message = Message::from_slice(message_hash)
            .map_err(|e| WalletError::Ethereum(format!("Invalid message hash: {}", e)))?;
        
        // Parse signature (this is simplified and would need adjustment for actual FROST signatures)
        let rec_id = signature[64];
        let recovery_id = secp256k1::ecdsa::RecoveryId::from_i32(rec_id as i32)
            .map_err(|e| WalletError::Ethereum(format!("Invalid recovery ID: {}", e)))?;
        
        let sig = secp256k1::ecdsa::RecoverableSignature::from_compact(&signature[0..64], recovery_id)
            .map_err(|e| WalletError::Ethereum(format!("Invalid signature: {}", e)))?;
        
        let public_key = secp.recover_ecdsa(&message, &sig)
            .map_err(|e| WalletError::Ethereum(format!("Failed to recover public key: {}", e)))?;
        
        // Convert to Ethereum address
        let public_key_bytes = public_key.serialize_uncompressed();
        let hash = keccak256(&public_key_bytes[1..]);
        let address = format!("0x{}", hex::encode(&hash[12..]));
        
        Ok(address)
    }
} 