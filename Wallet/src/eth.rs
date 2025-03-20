use crate::errors::{Result, WalletError};
use ethers::{
    prelude::*,
    types::{transaction::eip2718::TypedTransaction, TransactionRequest, U256, H256},
    utils::keccak256,
};
use std::str::FromStr;
use crate::key_manager::KeyShare;
use secp256k1::PublicKey;
use hex;

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
        
        // Debug: Check if we can get the latest block number
        match self.provider.get_block_number().await {
            Ok(block) => println!("Connected to Sepolia! Latest block: {}", block),
            Err(e) => println!("RPC connection error: {}", e),
        }
        
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
        from_address: &str,
        to_address: &str,
        value: U256,
        nonce: U256,
        gas_price: Option<U256>,
        gas_limit: Option<U256>,
        data: Option<Vec<u8>>,
    ) -> Result<(TypedTransaction, Vec<u8>)> {
        // Convert addresses
        let from_addr = from_address.parse::<Address>()
            .map_err(|e| WalletError::Ethereum(format!("Invalid from address: {}", e)))?;
        
        let to_addr = to_address.parse::<Address>()
            .map_err(|e| WalletError::Ethereum(format!("Invalid to address: {}", e)))?;
        
        // Create the transaction with EXPLICIT chain ID
        println!("Creating transaction with chain_id: {} (Sepolia)", self.chain_id);
        let mut tx = TransactionRequest::new()
            .from(from_addr)
            .to(to_addr)
            .value(value)
            .nonce(nonce)
            .chain_id(self.chain_id);  // <-- EXPLICIT chain ID set here
        
        // Set gas_limit - use default if not provided
        let gas_limit = gas_limit.unwrap_or_else(|| {
            println!("Using default gas limit of 21000");
            U256::from(21000)
        });
        tx = tx.gas(gas_limit);
        
        // Set gas_price - use default if not provided
        let gas_price = Some(U256::from(30_000_000_000u64)); // Force 30 Gwei
        println!("Setting gas price to 30 Gwei for faster inclusion");
        tx = tx.gas_price(gas_price.unwrap());
        
        // Set data if provided
        if let Some(data) = data {
            tx = tx.data(data);
        }
        
        // Convert to typed transaction
        let typed_tx = TypedTransaction::Legacy(tx);
        
        // Verify chain ID was set
        if typed_tx.chain_id().is_none() {
            println!("⚠️ ERROR: Chain ID still not set in transaction after explicit setting!");
            return Err(WalletError::Ethereum("Failed to set chain ID".to_string()));
        } else {
            println!("✅ Transaction created with chain_id: {}", typed_tx.chain_id().unwrap());
        }
        
        // Get transaction hash for signing
        let hash = typed_tx.sighash().to_fixed_bytes().to_vec();
        
        Ok((typed_tx, hash))
    }
    
    pub async fn send_transaction(&self, mut tx: TypedTransaction, signature: Vec<u8>) -> Result<H256> {
        // Add this debug code near the beginning
        if let Some(chain_id) = tx.chain_id() {
            println!("Transaction has chain_id: {}", chain_id);
            if chain_id.as_u64() != self.chain_id {
                println!("⚠️ WARNING: Transaction chain_id ({}) doesn't match client chain_id ({})",
                    chain_id, self.chain_id);
            }
        } else {
            println!("⚠️ WARNING: Transaction doesn't have chain_id set!");
            // We'll need to recreate the transaction with a chain ID
            // This creates a NEW transaction with the same parameters but with chain ID
            let mut tx_builder = TransactionRequest::new();
            
            // Copy all fields from the original transaction
            if let Some(from) = tx.from() { tx_builder = tx_builder.from(*from); }
            if let Some(to) = tx.to() { tx_builder = tx_builder.to(to.clone()); }
            if let Some(value) = tx.value() { tx_builder = tx_builder.value(*value); }
            if let Some(nonce) = tx.nonce() { tx_builder = tx_builder.nonce(nonce); }
            if let Some(gas) = tx.gas() { tx_builder = tx_builder.gas(gas); }
            if let Some(gas_price) = tx.gas_price() { tx_builder = tx_builder.gas_price(gas_price); }
            if let Some(data) = tx.data() { tx_builder = tx_builder.data(data.clone()); }
            
            // Add the chain ID
            tx_builder = tx_builder.chain_id(self.chain_id);
            
            // Convert back to TypedTransaction
            tx = TypedTransaction::Legacy(tx_builder);
            println!("Recreated transaction with chain_id: {}", self.chain_id);
        }
        
        // Add at the beginning of the send_transaction method
        println!("Using RPC URL: {}", self.provider.url());
        
        // Extract the r, s components from the signature
        if signature.len() != 65 {
            return Err(WalletError::Ethereum("Invalid signature length".to_string()));
        }
        
        let r = U256::from_big_endian(&signature[0..32]);
        let s = U256::from_big_endian(&signature[32..64]);
        
        // Print transaction details for debugging
        println!("Transaction details:");
        if let Some(from) = tx.from() {
            println!("Verifying From address: {:?}", from);
            
            // The from address is stored as an ethers::types::Address type
            // It's already validated by the ethers library when created
            // Just check if it looks reasonable
            let from_str = format!("{:?}", from);
            
            // The string representation of Address type might be in a different format
            // Let's extract just the hex part for proper comparison
            let address_hex = from_str.trim_start_matches("0x");
            if address_hex.len() != 40 {
                println!("⚠️ WARNING: From address has unusual length: {} chars", address_hex.len());
            } else {
                println!("✅ From address format is valid");
            }
        }
        if let Some(to) = tx.to() {
            println!("  To: {:?}", to);
            println!("  Is this the correct recipient? Please verify!");
        }
        if let Some(value) = tx.value() {
            println!("  Value: {} wei", value);
            let value_eth = value.as_u128() as f64 / 1_000_000_000_000_000_000.0;
            println!("  Value: {} ETH (approx)", value_eth);
        }
        if let Some(nonce) = tx.nonce() {
            println!("  Nonce: {}", nonce);
        }
        if let Some(gas) = tx.gas() {
            println!("  Gas limit: {}", gas);
        }
        if let Some(gas_price) = tx.gas_price() {
            println!("  Gas price: {} gwei", gas_price.as_u64() / 1_000_000_000);
        }
        
        // Instead of manually trying different v values, 
        // let the library handle it correctly:
        let signature = Signature {
            r,
            s,
            v: (self.chain_id * 2 + 35), // Always use EIP-155 format as u64
        };
        
        // The key is to ensure chain_id is set in the transaction itself
        let mut tx_with_chainid = tx.clone();
        if tx_with_chainid.chain_id().is_none() {
            println!("Setting missing chain_id in transaction");
            tx_with_chainid.set_chain_id(self.chain_id);
        }
        
        let signed_tx = tx_with_chainid.rlp_signed(&signature);
        
        // Print raw transaction bytes for debugging
        println!("Sending raw transaction with v={}: 0x{}", self.chain_id * 2 + 35, hex::encode(&signed_tx));
        println!("Transaction payload size: {} bytes", signed_tx.len());
        
        // Before sending, check the current network gas price
        match self.provider.get_gas_price().await {
            Ok(network_gas) => {
                let network_gwei = network_gas.as_u64() / 1_000_000_000;
                println!("Current network gas price: {} Gwei", network_gwei);
                
                if let Some(tx_gas) = tx.gas_price() {
                    let tx_gwei = tx_gas.as_u64() / 1_000_000_000;
                    if tx_gwei < network_gwei {
                        println!("⚠️ WARNING: Transaction gas price ({} Gwei) is below network average ({} Gwei)", 
                            tx_gwei, network_gwei);
                        println!("This may cause the transaction to be stuck or never mined");
                    }
                }
            },
            Err(e) => println!("Could not fetch network gas price: {}", e),
        }
        
        // Try to send with this v value
        match self.provider.send_raw_transaction(signed_tx.clone()).await {
            Ok(pending_tx) => {
                // Store hash before awaiting
                let tx_hash = pending_tx.tx_hash();
                let full_tx_hash_str = format!("0x{}", hex::encode(tx_hash.as_bytes()));
                
                println!("✅ Transaction accepted by RPC node! Hash: {}", full_tx_hash_str);
                
                // Try to get the transaction details immediately to verify it was indexed
                match self.provider.get_transaction(tx_hash).await {
                    Ok(Some(_)) => println!("✅ Transaction was indexed by the node!"),
                    Ok(None) => println!("⚠️ Transaction accepted but not yet indexed - this is normal"),
                    Err(e) => println!("⚠️ Error verifying transaction: {}", e),
                }
                
                println!("Check status at: https://sepolia.etherscan.io/tx/{}", full_tx_hash_str);
                return Ok(tx_hash);
            },
            Err(e) => {
                println!("Failed with v={}: {}", self.chain_id * 2 + 35, e);
                
                // If error mentions EIP-155, that's the issue
                if e.to_string().contains("EIP-155") {
                    println!("EIP-155 error detected - chain ID issue!");
                }
                
                Err(WalletError::Ethereum(format!("Failed to send transaction: {}", e)))
            }
        }
    }
    
    pub fn derive_address_from_public_key(public_key_hex: &str) -> Result<String> {
        println!("Deriving address from public key: {}", public_key_hex);
        
        // Parse the public key from hex
        let public_key_bytes = hex::decode(public_key_hex)
            .map_err(|e| WalletError::Ethereum(format!("Invalid public key hex: {}", e)))?;
            
        println!("Public key byte length: {}", public_key_bytes.len());
        
        // Parse as secp256k1 public key
        let public_key = PublicKey::from_slice(&public_key_bytes)
            .map_err(|e| WalletError::Ethereum(format!("Invalid public key: {}", e)))?;
        
        // Convert to uncompressed format required by Ethereum
        let uncompressed = public_key.serialize_uncompressed();
        println!("Uncompressed public key: 0x{} ({})", hex::encode(&uncompressed), uncompressed.len());
        
        // Take keccak256 hash of the key (excluding the format byte)
        let hash = keccak256(&uncompressed[1..]);
        
        // Last 20 bytes is the Ethereum address
        let address = format!("0x{}", hex::encode(&hash[12..]));
        println!("Calculated Ethereum address: {}", address);
        
        Ok(address)
    }

    #[allow(dead_code)]
    pub fn get_address_from_share(key_share: &KeyShare) -> Result<String> {
        Self::derive_address_from_public_key(&key_share.public_key)
    }

    // Add a diagnostic method
    pub async fn check_connection(&self) -> Result<()> {
        println!("Testing RPC connection to Sepolia...");
        
        match self.provider.get_block_number().await {
            Ok(block) => {
                println!("✅ RPC connection successful! Current block: {}", block);
                
                // Also check gas price to verify we can get network info
                match self.provider.get_gas_price().await {
                    Ok(gas) => println!("✅ Current network gas price: {} gwei", gas.as_u64() / 1_000_000_000),
                    Err(e) => println!("❌ Could not get gas price: {}", e),
                }
                
                Ok(())
            },
            Err(e) => {
                println!("❌ Failed to connect to RPC: {}", e);
                Err(WalletError::Ethereum(format!("RPC connection failed: {}", e)))
            }
        }
    }

    pub async fn check_transaction_status(&self, tx_hash: H256) -> Result<()> {
        // Format the full hash string
        let full_tx_hash_str = format!("0x{}", hex::encode(tx_hash.as_bytes()));
        println!("Checking status of transaction: {}", full_tx_hash_str);
        
        // Try to get transaction
        match self.provider.get_transaction(tx_hash).await {
            Ok(Some(tx)) => {
                println!("✅ Transaction found on-chain!");
                
                if let Some(block_number) = tx.block_number {
                    println!("✅ Included in block: {}", block_number);
                } else {
                    println!("⚠️ Transaction is still pending (not yet mined)");
                }
                
                println!("From: {}", tx.from);
                println!("To: {}", tx.to.unwrap_or_default());
                println!("Value: {} ETH", ethers::utils::format_ether(tx.value));
                println!("Gas price: {} gwei", tx.gas_price.unwrap_or_default().as_u64() / 1_000_000_000);
                
                Ok(())
            },
            Ok(None) => {
                println!("❌ Transaction NOT FOUND on-chain!");
                println!("This could mean:");
                println!("1. The transaction was never successfully submitted to the network");
                println!("2. The transaction is still pending in the mempool (not yet indexed)");
                println!("3. The RPC provider you're using doesn't have this transaction in its records");
                
                Err(WalletError::Ethereum(format!("Transaction not found: {}", full_tx_hash_str)))
            },
            Err(e) => {
                println!("❌ Error checking transaction: {}", e);
                Err(WalletError::Ethereum(format!("Failed to check transaction: {}", e)))
            }
        }
    }

    // Add this method to ensure correct chain ID
    pub async fn verify_chain_id(&self) -> Result<()> {
        match self.provider.get_chainid().await {
            Ok(chain_id) => {
                println!("Connected to chain with ID: {}", chain_id);
                
                if chain_id != U256::from(self.chain_id) {
                    println!("⚠️ WARNING: Connected to chain with ID {}, but expecting {}", chain_id, self.chain_id);
                    println!("This will cause signature verification to fail!");
                    return Err(WalletError::Ethereum(format!("Chain ID mismatch: expected {}, got {}", self.chain_id, chain_id)));
                }
                
                println!("✅ Chain ID verification successful");
                Ok(())
            },
            Err(e) => {
                println!("Failed to get chain ID: {}", e);
                Err(WalletError::Ethereum(format!("Failed to get chain ID: {}", e)))
            }
        }
    }
} 