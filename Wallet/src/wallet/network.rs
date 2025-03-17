use ethers::{
    prelude::*,
    utils::{keccak256, hex},
};
use super::PublicPackage;

pub fn derive_address(public: &PublicPackage) -> Address {
    let public_key = keccak256(&public.public_key[..]);
    Address::from_slice(&public_key[12..])
}

pub async fn create_transaction(
    rpc_url: &str,
    public: &PublicPackage,
    to: Address,
    value: U256,
) -> Result<(TransactionRequest, Vec<u8>)> {
    let provider = Provider::<Http>::try_from(rpc_url)?;
    let chain_id = provider.get_chainid().await?.as_u64();
    let nonce = provider.get_transaction_count(derive_address(public), None).await?;

    let tx = TransactionRequest::new()
        .to(to)
        .value(value)
        .chain_id(chain_id)
        .nonce(nonce)
        .gas(21000)
        .gas_price(provider.get_gas_price().await?);

    let sighash = tx.sighash();
    Ok((tx, sighash.to_vec()))
}

pub async fn send_transaction(
    rpc_url: &str,
    tx: TransactionRequest,
    signature: &[u8],
) -> Result<TxHash> {
    let provider = Provider::<Http>::try_from(rpc_url)?;
    let signature = signature.try_into()?;
    let signed_tx = tx.rlp_signed(&signature);
    provider.send_transaction(signed_tx, None).await
}
