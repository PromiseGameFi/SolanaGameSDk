use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Key generation error: {0}")]
    KeyGeneration(String),
    
    #[error("Signing error: {0}")]
    Signing(String),
    
    #[error("Threshold error: {0}")]
    Threshold(String),
    
    #[error("Ethereum error: {0}")]
    Ethereum(String),
    
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Hex decoding error: {0}")]
    HexDecoding(#[from] hex::FromHexError),
    
    #[error("Ethers provider error: {0}")]
    EthersProvider(String),
}

pub type Result<T> = std::result::Result<T, WalletError>; 