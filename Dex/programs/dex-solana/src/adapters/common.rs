use crate::error::ErrorCode;
use crate::{authority_pda, HopAccounts, BUMP_SA, SEED_SA, ZERO_ADDRESS};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::Instruction,
    program::{invoke, invoke_signed},
};
use anchor_spl::token_interface::TokenAccount;

pub trait DexProcessor {
    fn before_invoke(&self, _account_infos: &[AccountInfo]) -> Result<u64> {
        Ok(0)
    }

    fn after_invoke(&self, _account_infos: &[AccountInfo]) -> Result<u64> {
        Ok(0)
    }
}


pub fn before_check(
    swap_authority_pubkey: &AccountInfo,
    swap_source_token: Pubkey,
    swap_destination_token: Pubkey,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
) -> Result<()> {
    // check hop accounts
    if hop_accounts.from_account != ZERO_ADDRESS {
        require_keys_eq!(
            swap_source_token,
            hop_accounts.from_account,
            ErrorCode::InvalidHopAccounts
        );
        require_keys_eq!(
            swap_destination_token,
            hop_accounts.to_account,
            ErrorCode::InvalidHopAccounts
        );
    }
    if hop_accounts.last_to_account != ZERO_ADDRESS {
        require_keys_eq!(
            swap_source_token,
            hop_accounts.last_to_account,
            ErrorCode::InvalidHopFromAccount
        );
    }

    // check swap authority
    if !proxy_swap && hop == 0 {
        require!(
            swap_authority_pubkey.is_signer,
            ErrorCode::SwapAuthorityIsNotSigner
        );
    } else {
        require_keys_eq!(
            swap_authority_pubkey.key(),
            authority_pda::id(),
            ErrorCode::InvalidAuthorityPda
        );
    }
    Ok(())
}

pub fn invoke_process<'info, T: DexProcessor>(
    dex_processor: &T,
    account_infos: &[AccountInfo],
    swap_source_token: Pubkey,
    swap_destination_account: &mut InterfaceAccount<'info, TokenAccount>,
    hop_accounts: &mut HopAccounts,
    instruction: Instruction,
    hop: usize,
    offset: &mut usize,
    accounts_len: usize,
    proxy_swap: bool,
) -> Result<u64> {


    // check if pumpfun swap
    let before_destination_balance = swap_destination_account.amount;
    dex_processor.before_invoke(account_infos)?;
    if !proxy_swap && hop == 0 {
        invoke(&instruction, account_infos)?;
    } else {
        invoke_signed(&instruction, account_infos, &[&[SEED_SA, &[BUMP_SA]]])?;
    }

    // check if pumpfun swap
    dex_processor.after_invoke(account_infos)?;
    swap_destination_account.reload()?;
    let after_destination_balance = swap_destination_account.amount;
    *offset += accounts_len;
    hop_accounts.from_account = swap_source_token;
    hop_accounts.to_account = swap_destination_account.key();
    let amount_out = after_destination_balance
        .checked_sub(before_destination_balance)
        .ok_or(ErrorCode::CalculationError)?;
    Ok(amount_out)
}
