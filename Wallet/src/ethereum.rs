use curv::elliptic::curves::{secp256_k1::Secp256k1, Point};
use ethers::core::types::Address;
use sha3::{Digest, Keccak256};

/// Convert a secp256k1 public key to an Ethereum address
pub fn public_key_to_address(public_key: &Point<Secp256k1>) -> Address {
    // Get the uncompressed public key bytes
    let pk_bytes = public_key.to_bytes(false);
    
    // Remove the 0x04 prefix that indicates uncompressed format
    let pk_without_prefix = &pk_bytes[1..];
    
    // Keccak-256 hash the public key
    let mut hasher = Keccak256::new();
    hasher.update(pk_without_prefix);
    let hash = hasher.finalize();
    
    // Take the last 20 bytes of the hash as the address
    let mut address_bytes = [0u8; 20];
    address_bytes.copy_from_slice(&hash[12..32]);
    
    Address::from(address_bytes)
}