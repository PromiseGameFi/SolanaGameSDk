use super::{KeyShare, PublicPackage};
use multi_party_ecdsa::protocols::gg_2020::state_machine::keygen::Keygen;
use rand::thread_rng;
use std::{collections::HashMap, fs};

pub fn generate_shares(
    threshold: usize,
    total_shares: usize,
    output_dir: &str,
) -> anyhow::Result<()> {
    let mut rng = thread_rng();
    let params = Keygen::new(threshold, total_shares, &mut rng);
    let (secret_shares, public_shares) = params.generate()?;

    // Save public package
    let public_package = PublicPackage {
        threshold,
        total_shares,
        public_key: public_shares.group_key.serialize(),
        verification_shares: public_shares
            .verification_shares
            .iter()
            .map(|(k, v)| (*k, v.serialize().to_vec()))
            .collect(),
    };
    fs::write(
        format!("{}/public.json", output_dir),
        serde_json::to_string(&public_package)?,
    )?;

    // Save secret shares
    for (id, secret) in secret_shares {
        let share = KeyShare {
            id,
            private_share: secret.serialize(),
        };
        fs::write(
            format!("{}/share_{}.json", output_dir, id),
            serde_json::to_string(&share)?,
        )?;
    }

    Ok(())
}
