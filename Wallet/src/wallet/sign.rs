use super::{KeyShare, PublicKeyPackage};
use secp256k1::{Secp256k1, Message, ecdsa::Signature};
use anyhow::Result;

pub fn partial_sign(share: &KeyShare, message: &[u8]) -> Result<Vec<u8>> {
    let secp = Secp256k1::new();
    let msg = Message::from_digest_slice(message)?;
    let secret = multi_party_ecdsa::protocols::gg_2020::state_machine::keygen::LocalKey::deserialize(&share.private_share)?;
    
    let signature = secret.sign(&secp, &msg)?;
    Ok(signature.serialize_der().to_vec())
}

pub fn combine_signatures(
    public: &PublicKeyPackage,
    signatures: &[(usize, Vec<u8>)],
    message: &[u8],
) -> Result<Signature> {
    let secp = Secp256k1::new();
    let msg = Message::from_digest_slice(message)?;

    let mut sigs = vec![];
    for (id, sig_bytes) in signatures {
        let signature = Signature::from_der(sig_bytes)?;
        let share = multi_party_ecdsa::protocols::gg_2020::state_machine::keygen::LocalKey::deserialize(
            &public.verification_shares[id]
        )?;
        
        sigs.push((share, signature));
    }

    let combined_sig = multi_party_ecdsa::protocols::gg_2020::state_machine::keygen::SecretShares::combine_signatures(
        &secp,
        &sigs,
        &msg,
        public.threshold,
    )?;

    Ok(combined_sig)
}
