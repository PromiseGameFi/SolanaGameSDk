use super::{KeyShare, PublicKeyPackage};
use multi_party_ecdsa::protocols::gg_2020::state_machine::keygen::Keygen;
use rand::thread_rng;
use std::collections::HashMap;

pub fn generate_shares(
    threshold: usize,
    total_shares: usize,
) -> anyhow::Result<HashMap<usize, KeyShare>> {
    let mut rng = thread_rng();
    let params = Keygen::new(threshold, total_shares, &mut rng);
    let (secret_shares, public_shares) = params.generate()?;

    let mut shares = HashMap::new();
    for (id, secret) in secret_shares {
        let public_package = PublicKeyPackage {
            threshold,
            total_shares,
            public_key: public_shares.group_key.serialize(),
            verification_shares: public_shares.verification_shares
                .iter()
                .map(|(k,v)| (*k, v.serialize().to_vec()))
                .collect(),
        };

        shares.insert(id, KeyShare {
            id,
            private_share: secret.serialize(),
            public_package,
        });
    }

    Ok(shares)
}
