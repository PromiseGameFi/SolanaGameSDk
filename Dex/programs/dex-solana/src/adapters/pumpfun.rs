use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::spl_token::instruction::{close_account, sync_native};
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, TokenAccount};
use arrayref::array_ref;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::utils::transfer_sol_from_user;
use crate::{pumpfun_program, HopAccounts, PUMPFUN_BUY_SELECTOR, PUMPFUN_SELL_SELECTOR, ZERO_ADDRESS};

use super::common::DexProcessor;

const ARGS_LEN: usize = 24;

pub fn pumpfun_before_check(
    swap_authority_pubkey: &AccountInfo,
    swap_source_token: Pubkey,
    swap_source_token_account: TokenAccount,
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
            swap_source_token_account.owner,
            ErrorCode::InvalidAuthorityPda
        );
    }
    Ok(())
}



pub struct PumpfunBuyAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,
    pub global: &'info AccountInfo<'info>,
    pub fee_recipient: &'info AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub bonding_curve: &'info AccountInfo<'info>,
    pub associated_bonding_curve: &'info AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
}

const BUY_ACCOUNTS_LEN: usize = 13;


pub struct PumpfunBuyProcessor;
impl DexProcessor for PumpfunBuyProcessor { 
    fn before_invoke(&self, account_infos: &[AccountInfo]) -> Result<u64> {
        let source_token_account = account_infos.get(12).unwrap();
        let token_program = account_infos.get(8).unwrap();
        let user = account_infos.get(6).unwrap();

        let close_account_ix = close_account(
            &token_program.key(),    
            &source_token_account.key(),
            &user.key(),
            &user.key(),
            &[&user.key()],
        )?;

        invoke(&close_account_ix, &[source_token_account.to_account_info(), token_program.to_account_info(), user.to_account_info()])?;
        
        Ok(0)
    }
}

impl<'info> PumpfunBuyAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            global,
            fee_recipient,
            mint,
            bonding_curve,
            associated_bonding_curve,
            system_program,
            token_program,
            rent,
            event_authority,
            
        ]: &[AccountInfo<'info>; BUY_ACCOUNTS_LEN] = array_ref![accounts, offset, BUY_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            global,
            fee_recipient,
            mint: InterfaceAccount::try_from(mint)?,
            bonding_curve,
            associated_bonding_curve,
            system_program: Program::try_from(system_program)?,
            token_program: Program::try_from(token_program)?,
            rent,
            event_authority,
        })
    }

    fn cal_token_amount_out(&self, amount_in: u64) -> Result<u128> {
        let data = &self.bonding_curve.try_borrow_data()?;
        let virtual_token_reserves = u64::from_le_bytes(*array_ref![data, 8, 8]);
        let virtual_sol_reserves = u64::from_le_bytes(*array_ref![data, 16, 8]);
        let numerator = virtual_token_reserves as u128 * virtual_sol_reserves as u128;
        let denominator = virtual_sol_reserves as u128 + amount_in as u128;
        let amount_out = (virtual_token_reserves as u128)
            .checked_sub(numerator.checked_div(denominator).unwrap()).unwrap();
        Ok(amount_out as u128)
    }    
}


pub fn buy<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
) -> Result<u64> {
    msg!(
        "Dex::Pumpfun amount_in: {}, offset: {}",
        amount_in,
        offset
    );
    require!(
        remaining_accounts.len() >= *offset + BUY_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = PumpfunBuyAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &pumpfun_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    pumpfun_before_check(
        swap_accounts.swap_authority_pubkey,
        swap_accounts.swap_source_token.key(),
        *swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
    )?;

    let real_amount_in = amount_in.checked_mul(980009).unwrap().checked_div(1000000).unwrap();
    let amount_out = swap_accounts.cal_token_amount_out(real_amount_in)? as u64;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(PUMPFUN_BUY_SELECTOR); 
    data.extend_from_slice(&amount_out.to_le_bytes()); // token_amount_out
    data.extend_from_slice(&amount_in.to_le_bytes()); // max_amount_cost

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.global.key(), false),
        AccountMeta::new(swap_accounts.fee_recipient.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.associated_bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.rent.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false)
    ];

    let account_infos = vec![
        swap_accounts.global.to_account_info(),
        swap_accounts.fee_recipient.to_account_info(),
        swap_accounts.mint.to_account_info(),
        swap_accounts.bonding_curve.to_account_info(),
        swap_accounts.associated_bonding_curve.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.rent.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
    ];

    let instruction = Instruction{
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data,
    };
    
    let dex_processor = &PumpfunBuyProcessor;
    let amount_out = invoke_process(
        dex_processor,
        &account_infos,
        swap_accounts.swap_source_token.key(),
        &mut swap_accounts.swap_destination_token,
        hop_accounts,
        instruction,
        hop,
        offset,
        BUY_ACCOUNTS_LEN,
        false,
    )?;
    Ok(amount_out)
}



pub struct PumpfunSellProcessor {
    pub amount: u64,
}

impl DexProcessor for PumpfunSellProcessor {

    fn before_invoke(&self, account_infos: &[AccountInfo]) -> Result<u64> {
        let destination_token_account = account_infos.last().unwrap();
        let balance = destination_token_account.get_lamports();
        Ok(balance)
    }

    fn after_invoke(&self, account_infos: &[AccountInfo]) -> Result<u64> {
        let destination_token_account = account_infos.last().unwrap();
        let user = account_infos.get(6).unwrap();
        let token_program = account_infos.get(9).unwrap();
        transfer_sol_from_user(
            user.to_account_info(),
            destination_token_account.to_account_info(),
            self.amount,
        )?;
        
        let sync_native_ix = sync_native(
            &token_program.key(),
            &destination_token_account.key(),
        )?;
        invoke(&sync_native_ix, &[destination_token_account.to_account_info(), token_program.to_account_info()])?;
        Ok(self.amount)
    }
}


pub struct PumpfunSellAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,
    pub global: &'info AccountInfo<'info>,
    pub fee_recipient: &'info AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub bonding_curve: &'info AccountInfo<'info>,
    pub associated_bonding_curve: &'info AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub event_authority: &'info AccountInfo<'info>,
    
}

const SELL_ACCOUNTS_LEN: usize = 13;

impl<'info> PumpfunSellAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            global,
            fee_recipient,
            mint,
            bonding_curve,
            associated_bonding_curve,
            system_program,
            associated_token_program,
            token_program,
            event_authority,
       
        ]: &[AccountInfo<'info>; SELL_ACCOUNTS_LEN] = array_ref![accounts, offset, SELL_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            global,
            fee_recipient,
            mint: InterfaceAccount::try_from(mint)?,
            bonding_curve,
            associated_bonding_curve,
            system_program: Program::try_from(system_program)?,
            associated_token_program: Program::try_from(associated_token_program)?,
            token_program: Program::try_from(token_program)?,
            event_authority,
      
        })
    }

    fn cal_sol_amount_out(&self, token_amount_in: u64) -> Result<u128> {
        let data = &self.bonding_curve.try_borrow_data()?;
        let virtual_token_reserves = u64::from_le_bytes(*array_ref![data, 8, 8]);
        let virtual_sol_reserves = u64::from_le_bytes(*array_ref![data, 16, 8]);
        let numerator = virtual_token_reserves as u128 * virtual_sol_reserves as u128;

        let sol_amount_out = numerator.checked_div(virtual_token_reserves as u128 - token_amount_in as u128).unwrap() - virtual_sol_reserves as u128;
        Ok(sol_amount_out)
    }
}



pub fn sell<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
) -> Result<u64> {
    msg!(
        "Dex::Pumpfun amount_in: {}, offset: {}",
        amount_in,
        offset
    );
    require!(
        remaining_accounts.len() >= *offset + SELL_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = PumpfunSellAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &pumpfun_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    before_check(
        swap_accounts.swap_authority_pubkey, 
        swap_accounts.swap_source_token.key(),
        swap_accounts.swap_destination_token.key(), 
        hop_accounts, 
        hop, 
        proxy_swap
    )?;

    let sol_amount_out = swap_accounts.cal_sol_amount_out(amount_in)? as u64;
    let min_sol_amount_out = sol_amount_out.checked_mul(980009).unwrap().checked_div(1000000).unwrap();

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(PUMPFUN_SELL_SELECTOR); 
    data.extend_from_slice(&amount_in.to_le_bytes()); // token_amount_in
    data.extend_from_slice(&min_sol_amount_out.to_le_bytes()); // min_sol_amount_out
   
    
    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.global.key(), false),
        AccountMeta::new(swap_accounts.fee_recipient.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.associated_bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.global.to_account_info(),
        swap_accounts.fee_recipient.to_account_info(),
        swap_accounts.mint.to_account_info(),
        swap_accounts.bonding_curve.to_account_info(),
        swap_accounts.associated_bonding_curve.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.associated_token_program.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
    ];

    let instruction = Instruction{
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data,
    };
    let dex_processor = &PumpfunSellProcessor{amount: min_sol_amount_out};
   
    let amount_out = invoke_process(
        dex_processor,
        &account_infos,
        swap_accounts.swap_source_token.key(),
        &mut swap_accounts.swap_destination_token,
        hop_accounts,
        instruction,
        hop,
        offset,
        SELL_ACCOUNTS_LEN,
        false,
    )?;
    Ok(amount_out)
}