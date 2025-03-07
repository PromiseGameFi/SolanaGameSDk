use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{
    meteora_dlmm_program, meteora_dynamicpool_program, HopAccounts, SWAP_SELECTOR, ZERO_ADDRESS,
};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, TokenAccount};
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 24;

pub struct MeteoraDynamicPoolProcessor;
impl DexProcessor for MeteoraDynamicPoolProcessor {}

pub struct MeteoraDynamicPoolAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool: &'info AccountInfo<'info>,
    pub a_vault: &'info AccountInfo<'info>,
    pub b_vault: &'info AccountInfo<'info>,
    pub a_token_vault: &'info AccountInfo<'info>,
    pub b_token_vault: &'info AccountInfo<'info>,
    pub a_vault_lp_mint: InterfaceAccount<'info, Mint>,
    pub b_vault_lp_mint: InterfaceAccount<'info, Mint>,
    pub a_vault_lp: InterfaceAccount<'info, TokenAccount>,
    pub b_vault_lp: InterfaceAccount<'info, TokenAccount>,
    pub admin_token_fee: &'info AccountInfo<'info>,
    pub vault_program: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}
const ACCOUNTS_LEN: usize = 16;

pub struct MeteoraDlmmAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub lb_pair: &'info AccountInfo<'info>,
    pub bin_array_bitmap_extension: &'info AccountInfo<'info>,
    pub reserve_x: InterfaceAccount<'info, TokenAccount>,
    pub reserve_y: InterfaceAccount<'info, TokenAccount>,
    pub token_x_mint: InterfaceAccount<'info, Mint>,
    pub token_y_mint: InterfaceAccount<'info, Mint>,
    pub oracle: &'info AccountInfo<'info>,
    pub host_fee_in: &'info AccountInfo<'info>,
    pub token_x_program: &'info AccountInfo<'info>,
    pub token_y_program: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
    pub bin_array0: &'info AccountInfo<'info>,
    pub bin_array1: &'info AccountInfo<'info>,
    pub bin_array2: &'info AccountInfo<'info>,
}
const DLMM_ACCOUNTS_LEN: usize = 18;

impl<'info> MeteoraDynamicPoolAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool,
            a_vault,
            b_vault,
            a_token_vault,
            b_token_vault,
            a_vault_lp_mint,
            b_vault_lp_mint,
            a_vault_lp,
            b_vault_lp,
            admin_token_fee,
            vault_program,
            token_program,
      ]: & [AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool,
            a_vault,
            b_vault,
            a_token_vault,
            b_token_vault,
            a_vault_lp_mint: InterfaceAccount::try_from(a_vault_lp_mint)?,
            b_vault_lp_mint: InterfaceAccount::try_from(b_vault_lp_mint)?,
            a_vault_lp: InterfaceAccount::try_from(a_vault_lp)?,
            b_vault_lp: InterfaceAccount::try_from(b_vault_lp)?,
            admin_token_fee,
            vault_program,
            token_program: Program::try_from(token_program)?,
        })
    }
}

impl<'info> MeteoraDlmmAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            lb_pair,
            bin_array_bitmap_extension,
            reserve_x,
            reserve_y,
            token_x_mint,
            token_y_mint,
            oracle,
            host_fee_in,
            token_x_program,
            token_y_program,
            event_authority,
            bin_array0,
            bin_array1,
            bin_array2,
        ]: & [AccountInfo<'info>; DLMM_ACCOUNTS_LEN] = array_ref![accounts, offset, DLMM_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            lb_pair,
            bin_array_bitmap_extension,
            reserve_x: InterfaceAccount::try_from(reserve_x)?,
            reserve_y: InterfaceAccount::try_from(reserve_y)?,
            token_x_mint: InterfaceAccount::try_from(token_x_mint)?,
            token_y_mint: InterfaceAccount::try_from(token_y_mint)?,
            oracle,
            host_fee_in,
            token_x_program,
            token_y_program,
            event_authority,
            bin_array0,
            bin_array1,
            bin_array2,
        })
    }
}

pub fn swap<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
) -> Result<u64> {
    msg!(
        "Dex::MeteoraSwap amount_in: {}, offset: {}",
        amount_in,
        offset
    );
    require!(
        remaining_accounts.len() >= *offset + ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts =
        MeteoraDynamicPoolAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_dynamicpool_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    // check hop accounts & swap authority
    let swap_source_token = swap_accounts.swap_source_token.key();
    let swap_destination_token = swap_accounts.swap_destination_token.key();
    before_check(
        &swap_accounts.swap_authority_pubkey,
        swap_source_token,
        swap_destination_token,
        hop_accounts,
        hop,
        proxy_swap,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new(swap_accounts.a_vault.key(), false),
        AccountMeta::new(swap_accounts.b_vault.key(), false),
        AccountMeta::new(swap_accounts.a_token_vault.key(), false),
        AccountMeta::new(swap_accounts.b_token_vault.key(), false),
        AccountMeta::new(swap_accounts.a_vault_lp_mint.key(), false),
        AccountMeta::new(swap_accounts.b_vault_lp_mint.key(), false),
        AccountMeta::new(swap_accounts.a_vault_lp.key(), false),
        AccountMeta::new(swap_accounts.b_vault_lp.key(), false),
        AccountMeta::new(swap_accounts.admin_token_fee.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.vault_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pool.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.a_vault.to_account_info(),
        swap_accounts.b_vault.to_account_info(),
        swap_accounts.a_token_vault.to_account_info(),
        swap_accounts.b_token_vault.to_account_info(),
        swap_accounts.a_vault_lp_mint.to_account_info(),
        swap_accounts.b_vault_lp_mint.to_account_info(),
        swap_accounts.a_vault_lp.to_account_info(),
        swap_accounts.b_vault_lp.to_account_info(),
        swap_accounts.admin_token_fee.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.vault_program.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data,
    };  

    let dex_processor = &MeteoraDynamicPoolProcessor;
    let amount_out = invoke_process(
        dex_processor,
        &account_infos,
        swap_source_token,
        &mut swap_accounts.swap_destination_token,
        hop_accounts,
        instruction,
        hop,
        offset,
        ACCOUNTS_LEN,
        proxy_swap,
    )?;
    Ok(amount_out)
}

pub fn swap_dlmm<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
) -> Result<u64> {
    msg!(
        "Dex::MeteoraDlmm amount_in: {}, offset: {}",
        amount_in,
        offset
    );
    require!(
        remaining_accounts.len() >= *offset + DLMM_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = MeteoraDlmmAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_dlmm_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    // check hop accounts & swap authority
    let swap_source_token = swap_accounts.swap_source_token.key();
    let swap_destination_token = swap_accounts.swap_destination_token.key();
    before_check(
        &swap_accounts.swap_authority_pubkey,
        swap_source_token,
        swap_destination_token,
        hop_accounts,
        hop,
        proxy_swap,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new(swap_accounts.lb_pair.key(), false),
        AccountMeta::new_readonly(swap_accounts.bin_array_bitmap_extension.key(), false),
        AccountMeta::new(swap_accounts.reserve_x.key(), false),
        AccountMeta::new(swap_accounts.reserve_y.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new_readonly(swap_accounts.token_x_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_y_mint.key(), false),
        AccountMeta::new(swap_accounts.oracle.key(), false),
        AccountMeta::new(swap_accounts.host_fee_in.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.token_x_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_y_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new(swap_accounts.bin_array0.key(), false),
    ];

    let mut account_infos = vec![
        swap_accounts.lb_pair.to_account_info(),
        swap_accounts.bin_array_bitmap_extension.to_account_info(),
        swap_accounts.reserve_x.to_account_info(),
        swap_accounts.reserve_y.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.token_x_mint.to_account_info(),
        swap_accounts.token_y_mint.to_account_info(),
        swap_accounts.oracle.to_account_info(),
        swap_accounts.host_fee_in.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.token_x_program.to_account_info(),
        swap_accounts.token_y_program.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.bin_array0.to_account_info(),
    ];

    let bin_array1 = swap_accounts.bin_array1.key();
    let bin_array2 = swap_accounts.bin_array2.key();
    if bin_array1 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(bin_array1, false));
        account_infos.push(swap_accounts.bin_array1.to_account_info());
    }
    if bin_array2 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(bin_array2, false));
        account_infos.push(swap_accounts.bin_array2.to_account_info());
    }

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data,
    };

    let dex_processor = &MeteoraDynamicPoolProcessor;
    let amount_out = invoke_process(
        dex_processor,
        &account_infos,
        swap_source_token,
        &mut swap_accounts.swap_destination_token,
        hop_accounts,
        instruction,
        hop,
        offset,
        DLMM_ACCOUNTS_LEN,
        proxy_swap,
    )?;
    Ok(amount_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_pack_swap_instruction() {
        let amount_in = 100u64;
        let mut data = Vec::with_capacity(ARGS_LEN);
        data.extend_from_slice(SWAP_SELECTOR);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&1u64.to_le_bytes());

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_LEN);
    }
}
