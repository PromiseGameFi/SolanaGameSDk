use super::{KeyShare, PublicPackage};
use secp256k1::{Secp256k1, Message, ecdsa::Signature};
use multi_party_ecdsa::protocols::gg_2020::state_machine::keygen::LocalKey;
use anyhow::Result;

pub fn partial_sign(share: &KeyShare, message: &[u8]) -> Result<Vec<u8>> {
    let secp = Secp256k1::new();
    let msg = Message::from_digest_slice(message)?;
    let secret = LocalKey::deserialize(&share.private_share)?;
    
    let signature = secret.sign(&secp, &msg)?;
    Ok(signature.serialize_der().to_vec())
}

pub fn combine_signatures(
    public: &PublicPackage,
    signatures: &[(usize, Vec<u8>)],
    message: &[u8],
) -> Result<Signature> {
    let secp = Secp256k1::new();
    let msg = Message::from_digest_slice(message)?;

    let mut sigs = vec![];
    for (id, sig_bytes) in signatures {
        let signature = Signature::from_der(sig_bytes)?;
        let verification_key = LocalKey::deserialize(
            &public.verification_shares[id]
        )?.verification_key;
        
        // Verify each partial signature before combining
        verification_key.verify(&secp, &msg, &signature)?;
        
        sigs.push((verification_key, signature));
    }

    let combined_sig = LocalKey::combine_signatures(
        &secp,
        &sigs,
        &msg,
        public.threshold,
    )?;

    Ok(combined_sig)
}
