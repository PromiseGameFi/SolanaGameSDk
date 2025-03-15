use anyhow::Result;
use ethers::core::types::{Address, Bytes, TransactionRequest, U256};
use ethers::utils::rlp::{self, Encodable, RlpStream};
use sha3::{Digest, Keccak256};

#[derive(Debug, Clone)]
pub struct EthereumTransaction {
    pub nonce: U256,
    pub gas_price: U256,
    pub gas_limit: U256,
    pub to: Option<Address>,
    pub value: U256,
    pub data: Bytes,
    pub chain_id: u64,
}

impl EthereumTransaction {
    /// Hash the transaction according to EIP-155
    pub fn hash(&self) -> Vec<u8> {
        let encoded = self.rlp_encode_for_signing();
        let mut hasher = Keccak256::new();
        hasher.update(&encoded);
        hasher.finalize().to_vec()
    }
    
    /// RLP encode the transaction for signing (EIP-155)
    fn rlp_encode_for_signing(&self) -> Vec<u8> {
        let mut stream = RlpStream::new();
        stream.begin_list(9);
        
        // Encode transaction fields
        stream.append(&self.nonce);
        stream.append(&self.gas_price);
        stream.append(&self.gas_limit);
        
        match self.to {
            Some(address) => stream.append(&address),
            None => stream.append(&""),
        }
        
        stream.append(&self.value);
        stream.append(&self.data.0);
        
        // EIP-155: Append chain ID and empty r, s values
        stream.append(&self.chain_id);
        stream.append(&0u8);
        stream.append(&0u8);
        
        stream.out().to_vec()
    }
    
    /// Convert to ethers TransactionRequest
    pub fn to_transaction_request(&self) -> TransactionRequest {
        let mut request = TransactionRequest::new()
            .nonce(self.nonce)
            .gas_price(self.gas_price)
            .gas(self.gas_limit)
            .value(self.value)
            .data(self.data.clone())
            .chain_id(self.chain_id);
        
        if let Some(to) = self.to {
            request = request.to(to);
        }
        
        request
    }
    
    /// Apply a signature to the transaction and return the RLP encoded signed transaction
    pub fn apply_signature(&self, signature: Vec<u8>) -> Result<Bytes> {
        if signature.len() != 65 {
            anyhow::bail!("Invalid signature length: expected 65 bytes");
        }
        
        let r = U256::from_big_endian(&signature[0..32]);
        let s = U256::from_big_endian(&signature[32..64]);
        let v = U256::from(signature[64]);
        
        let mut stream = RlpStream::new();
        stream.begin_list(9);
        
        stream.append(&self.nonce);
        stream.append(&self.gas_price);
        stream.append(&self.gas_limit);
        
        match self.to {
            Some(address) => stream.append(&address),
            None => stream.append(&""),
        }
        
        stream.append(&self.value);
        stream.append(&self.data.0);
        
        // Append v, r, s
        stream.append(&v);
        stream.append(&r);
        stream.append(&s);
        
        Ok(Bytes::from(stream.out().to_vec()))
    }
}