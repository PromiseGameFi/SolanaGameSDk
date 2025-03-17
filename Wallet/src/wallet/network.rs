use ethers::{
    prelude::*,
    utils::{keccak256, hex},
};
use super::PublicKeyPackage;

pub fn derive_address(public: &PublicKeyPackage) -> Address {
    let public_key = keccak256(&public.public_key[..]);
    Address::from_slice(&public_key[12..])
}

pub async fn send_transaction(
    rpc_url: &str,
    public: &PublicKeyPackage,
    signature: &[u8],
    tx: TransactionRequest,
) -> Result<TxHash, ProviderError> {
    let provider = Provider::<Http>::try_from(rpc_url)?;
    let signature = signature.try_into()?;
    let signed_tx = tx.rlp_signed(&signature);
    provider.send_transaction(signed_tx, None).await
}

pub fn construct_transaction(
    to: Address,
    value: U256,
    nonce: U256,
    chain_id: u64,
) -> (TransactionRequest, Vec<u8>) {
    let tx = TransactionRequest::new()
        .to(to)
        .value(value)
        .chain_id(chain_id)
        .nonce(nonce);
    
    let sighash = tx.sighash();
    (tx, sighash.to_vec())
}
