use anyhow::{Context, Result};
use curv::arithmetic::Converter;
use curv::elliptic::curves::{Point, Scalar, Secp256k1};
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::{
    KeyGenBroadcastMessage1, KeyGenDecommitMessage1, Keys, Parameters,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone)]
pub struct KeyShare {
    pub index: usize,
    pub threshold: usize,
    pub total_shares: usize,
    pub keys: Keys,
    pub public_key: Point<Secp256k1>,
    pub vss_scheme_vec: Vec<Vec<u8>>,
}

impl KeyShare {
    pub fn get_address(&self) -> String {
        // Convert public key to Ethereum address
        let pk_bytes = self.public_key.to_bytes(false).expect("Invalid public key");
        let pk_hash = {
            let mut hasher = tiny_keccak::Keccak::v256();
            let mut hash = [0u8; 32];
            hasher.update(&pk_bytes[1..]); // Skip the 0x04 prefix
            hasher.finalize(&mut hash);
            hash
        };
        
        // Take last 20 bytes of hash as Ethereum address
        format!("0x{}", hex::encode(&pk_hash[12..32]))
    }
}

pub fn generate_shares(total: usize, threshold: usize, output_dir: &Path) -> Result<()> {
    if threshold > total {
        anyhow::bail!("Threshold cannot be greater than total shares");
    }
    
    if threshold < 2 {
        anyhow::bail!("Threshold must be at least 2");
    }
    
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;
    
    // Initialize parameters for the MPC-TSS scheme
    let params = Parameters {
        threshold,
        share_count: total,
    };
    
    // Generate a master secret using a secure RNG
    let mut rng = OsRng;
    
    // These vectors will store the data from the first round of the protocol
    let mut bc1_vec = Vec::new();
    let mut decom_vec = Vec::new();
    let mut keys_vec = Vec::new();
    
    // First round of key generation (commitment phase)
    for i in 1..=total {
        // Replace the following with the correct function calls based on the library's documentation
        // let (bc1, decom1, keys) = multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::phase1_broadcast_phase2_distribute_commitments_and_keys(
        //     &params, i, &mut rng
        // )?;
        // Adjust this part according to the available functions in the library
        
        bc1_vec.push(bc1);
        decom_vec.push(decom1);
        keys_vec.push(keys);
    }
    
    // Second round of key generation (verification and VSS phase)
    let mut vss_scheme_vec_all = Vec::new();
    let mut secret_shares_vec_all = Vec::new();
    
    for i in 1..=total {
        let (vss_schemes, secret_shares, _indices) = multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::phase2_verify_commitments_phase3_distribute_shares(
            &params, &keys_vec[i-1], &decom_vec, &bc1_vec
        )?;
        
        vss_scheme_vec_all.push(vss_schemes);
        secret_shares_vec_all.push(secret_shares);
    }
    
    // Final phase: derive the public key and store shares
    let pk_vec = (1..=total)
        .map(|i| {
            let mut vss_schemes_with_defaults = Vec::new();
            let mut secret_shares_with_defaults = Vec::new();
            
            for j in 1..=total {
                let vss_schemes_j = vss_scheme_vec_all[j-1].clone();
                let secret_shares_j = secret_shares_vec_all[j-1].clone();
                
                vss_schemes_with_defaults.push(vss_schemes_j[i-1].clone());
                secret_shares_with_defaults.push(secret_shares_j[i-1].clone());
            }
            
            multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::phase3_reconstruct_and_set_private_key(
                &params, i, &keys_vec[i-1], &vss_schemes_with_defaults, &secret_shares_with_defaults
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    
    // Verify that all parties have the same public key
    let first_pk = pk_vec[0].clone();
    for pk in pk_vec.iter().skip(1) {
        if *pk != first_pk {
            anyhow::bail!("Public key mismatch between parties");
        }
    }
    
    // Store shares as JSON files
    for i in 1..=total {
        let serialized_vss = serde_json::to_vec(&vss_scheme_vec_all[i-1])?;
        
        let share = KeyShare {
            index: i,
            threshold,
            total_shares: total,
            keys: keys_vec[i-1].clone(),
            public_key: first_pk.clone(),
            vss_scheme_vec: serialized_vss,
        };
        
        let share_path = output_dir.join(format!("share_{}.json", i));
        let share_json = serde_json::to_string_pretty(&share)?;
        fs::write(&share_path, share_json)?;
    }
    
    // Output the generated Ethereum address for reference
    let ethereum_address = KeyShare {
        index: 1,
        threshold,
        total_shares: total,
        keys: keys_vec[0].clone(),
        public_key: first_pk.clone(),
        vss_scheme_vec: Vec::new(),
    }.get_address();
    
    println!("Generated a {}-of-{} MPC-TSS wallet", threshold, total);
    println!("Ethereum address: {}", ethereum_address);
    println!("Key shares saved to: {}", output_dir.display());
    
    Ok(())
}

pub fn load_share(path: &Path) -> Result<KeyShare> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read key share from {}", path.display()))?;
    
    let share: KeyShare = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse key share from {}", path.display()))?;
    
    Ok(share)
}