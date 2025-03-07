use anchor_lang::prelude::*;

#[constant]
pub const SEED_SA: &[u8] = b"okx_sa";
pub const BUMP_SA: u8 = 251;
pub const COMMISSION_RATE_LIMIT: u16 = 300;
pub const COMMISSION_DENOMINATOR: u64 = 10000;
pub const MAX_HOPS: usize = 3;
pub const TOTAL_WEIGHT: u8 = 100;

pub const SWAP_SELECTOR: &[u8; 8] = &[248, 198, 158, 145, 225, 117, 135, 200];
pub const CPSWAP_SELECTOR: &[u8; 8] = &[143, 190, 90, 218, 196, 30, 51, 222];
pub const SWAP_V2_SELECTOR: &[u8; 8] = &[43, 4, 237, 11, 26, 201, 30, 98];
pub const PLACE_TAKE_ORDER_SELECTOR: &[u8; 8] = &[3, 44, 71, 3, 26, 199, 203, 85];
pub const BRIDGE_TO_LOG_SELECTOR: &[u8; 8] = &[212, 189, 176, 218, 196, 135, 64, 122];
pub const ZERO_ADDRESS: Pubkey = Pubkey::new_from_array([0u8; 32]);

pub const PUMPFUN_BUY_SELECTOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
pub const PUMPFUN_SELL_SELECTOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];

pub mod authority_pda {
    use anchor_lang::declare_id;
    declare_id!("HV1KXxWFaSeriyFvXyx48FqG9BoFbfinB8njCJonqP7K");
    // declare_id!("4DwLmWvMyWPPKa8jhmW6AZKGctUMe7GxAWrb2Wcw8ZUa"); //pre_deploy
}

pub mod okx_bridge_program {
    use anchor_lang::declare_id;
    declare_id!("okxBd18urPbBi2vsExxUDArzQNcju2DugV9Mt46BxYE");
}

pub mod token_program {
    use anchor_lang::declare_id;
    declare_id!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
}

pub mod wsol_program {
    use anchor_lang::declare_id;
    declare_id!("So11111111111111111111111111111111111111112");
}

pub mod raydium_swap_program {
    use anchor_lang::declare_id;
    declare_id!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
}

pub mod raydium_stable_program {
    use anchor_lang::declare_id;
    declare_id!("5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h");
}

pub mod raydium_clmm_program {
    use anchor_lang::declare_id;
    declare_id!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
}

pub mod raydium_cpmm_program {
    use anchor_lang::declare_id;
    declare_id!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
}

pub mod aldrin_v1_program {
    use anchor_lang::declare_id;
    declare_id!("AMM55ShdkoGRB5jVYPjWziwk8m5MpwyDgsMWHaMSQWH6");
}

pub mod aldrin_v2_program {
    use anchor_lang::declare_id;
    declare_id!("CURVGoZn8zycx6FXwwevgBTB2gVvdbGTEpvMJDbgs2t4");
}

pub mod whirlpool_program {
    use anchor_lang::declare_id;
    declare_id!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
}

pub mod meteora_dynamicpool_program {
    use anchor_lang::declare_id;
    declare_id!("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB");
}

pub mod meteora_dlmm_program {
    use anchor_lang::declare_id;
    declare_id!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");
}

pub mod lifinity_v1pool_program {
    use anchor_lang::declare_id;
    declare_id!("EewxydAPCCVuNEyrVN68PuSYdQ7wKn27V9Gjeoi8dy3S");
}

pub mod lifinity_v2pool_program {
    use anchor_lang::declare_id;
    declare_id!("2wT8Yq49kHgDzXuPxZSaeLaH1qbmGXtEyPy64bL7aD3c");
}

pub mod flux_beam_program {
    use anchor_lang::declare_id;
    declare_id!("FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X");
}

pub mod openbookv2_program {
    use anchor_lang::declare_id;
    declare_id!("opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb");
}

pub mod phoenix_program {
    use anchor_lang::declare_id;
    declare_id!("PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY");
}

pub mod obric_v2_program {
    use anchor_lang::declare_id;
    declare_id!("obriQD1zbpyLz95G5n7nJe6a4DPjpFwa5XYPoNm113y");
}

pub mod sanctum_program {
    use anchor_lang::declare_id;
    declare_id!("5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx");
}

pub mod pumpfun_program {
    use anchor_lang::declare_id;
    declare_id!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
}
