pub mod keygen;
pub mod sign;
pub mod network;

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyShare {
    pub id: usize,
    pub private_share: Vec<u8>,
    pub public_package: PublicKeyPackage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyPackage {
    pub threshold: usize,
    pub total_shares: usize,
    pub public_key: [u8; 64],
    pub verification_shares: HashMap<usize, Vec<u8>>,
}
