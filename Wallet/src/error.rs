use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    
    #[error("Cryptographic error: {0}")]
    Crypto(String),
    
    #[error("Ethereum interaction error: {0}")]
    Ethereum(String),
    
    #[error("Provider error: {0}")]
    Provider(#[from] ethers::providers::ProviderError),
    
    #[error("Address parsing error: {0}")]
    AddressParsing(#[from] ethers::core::types::address::ConversionError),
    
    #[error("Private key error: {0}")]
    PrivateKey(#[from] ethers::signers::WalletError),
    
    #[error("Contract error: {0}")]
    Contract(#[from] ethers::contract::ContractError<ethers::providers::Provider<ethers::providers::Http>>),
}

impl From<ethers::utils::ConversionError> for WalletError {
    fn from(err: ethers::utils::ConversionError) -> Self {
        WalletError::Ethereum(err.to_string())
    }
} 