use crate::error::ErrorCode;
use crate::instructions::from_swap::cpi_bridge_to_log;
use crate::utils::token::{transfer_sol_from_user, transfer_token_from_user};
use crate::{
    swap_process, wsol_program, BridgeToArgs, SwapArgs, COMMISSION_DENOMINATOR,
    COMMISSION_RATE_LIMIT,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::Token,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

#[derive(Accounts)]
pub struct CommissionSOLFromSwapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
    )]
    pub source_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = destination_mint,
    )]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    pub source_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub destination_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: bridge_program
    #[account(address = crate::okx_bridge_program::id())]
    pub bridge_program: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub commission_account: SystemAccount<'info>,
}

pub fn commission_sol_from_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSOLFromSwapAccounts<'a>>,
    args: SwapArgs,
    commission_rate: u16,
    bridge_to_args: BridgeToArgs,
    offset: u8,
    len: u8,
) -> Result<()> {
    require!(
        commission_rate > 0 && commission_rate <= COMMISSION_RATE_LIMIT,
        ErrorCode::InvalidCommissionRate
    );
    require!(
        ctx.accounts.source_mint.key() == wsol_program::id(),
        ErrorCode::InvalidCommissionTokenAccount
    );

    let commission_amount = args
        .amount_in
        .checked_mul(commission_rate as u64)
        .ok_or(ErrorCode::CalculationError)?
        .checked_div(COMMISSION_DENOMINATOR - commission_rate as u64)
        .ok_or(ErrorCode::CalculationError)?;

    let amount_out = swap_process(
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        ctx.remaining_accounts,
        args,
        bridge_to_args.order_id,
        false,
    )?;

    // Transfer commission_amount
    transfer_sol_from_user(
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.commission_account.to_account_info(),
        commission_amount,
    )?;
    msg!(
        "commission_direction: {:?}, commission_amount: {:?}",
        true,
        commission_amount
    );

    // CPI bridge_to_log
    cpi_bridge_to_log(
        bridge_to_args,
        amount_out,
        offset,
        len,
        &ctx.accounts.bridge_program,
        &ctx.accounts.payer,
        &ctx.accounts.destination_token_account,
        &ctx.accounts.destination_mint,
        &ctx.accounts.associated_token_program,
        &ctx.accounts.token_program,
        &ctx.accounts.token_2022_program,
        &ctx.accounts.system_program,
        &ctx.remaining_accounts,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct CommissionSPLFromSwapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
    )]
    pub source_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = destination_mint,
    )]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    pub source_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub destination_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: bridge_program
    #[account(address = crate::okx_bridge_program::id())]
    pub bridge_program: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,

    #[account(
        mut,
        token::token_program = token_program,
    )]
    pub commission_token_account: InterfaceAccount<'info, TokenAccount>,
}

pub fn commission_spl_from_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSPLFromSwapAccounts<'a>>,
    args: SwapArgs,
    commission_rate: u16,
    bridge_to_args: BridgeToArgs,
    offset: u8,
    len: u8,
) -> Result<()> {
    require!(
        commission_rate > 0 && commission_rate <= COMMISSION_RATE_LIMIT,
        ErrorCode::InvalidCommissionRate
    );
    require!(
        ctx.accounts.commission_token_account.mint == ctx.accounts.source_mint.key(),
        ErrorCode::InvalidCommissionTokenAccount
    );

    let commission_amount = args
        .amount_in
        .checked_mul(commission_rate as u64)
        .ok_or(ErrorCode::CalculationError)?
        .checked_div(COMMISSION_DENOMINATOR - commission_rate as u64)
        .ok_or(ErrorCode::CalculationError)?;

    let amount_out = swap_process(
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        ctx.remaining_accounts,
        args,
        bridge_to_args.order_id,
        false,
    )?;

    // Transfer commission_amount
    let commission_token_program =
        if *ctx.accounts.source_mint.to_account_info().owner == Token2022::id() {
            ctx.accounts.token_2022_program.to_account_info()
        } else {
            ctx.accounts.token_program.to_account_info()
        };
    transfer_token_from_user(
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.source_token_account.to_account_info(),
        ctx.accounts.commission_token_account.to_account_info(),
        ctx.accounts.source_mint.to_account_info(),
        commission_token_program,
        commission_amount,
        ctx.accounts.source_mint.decimals,
    )?;
    msg!(
        "commission_direction: {:?}, commission_amount: {:?}",
        true,
        commission_amount
    );

    // CPI bridge_to_log
    cpi_bridge_to_log(
        bridge_to_args,
        amount_out,
        offset,
        len,
        &ctx.accounts.bridge_program,
        &ctx.accounts.payer,
        &ctx.accounts.destination_token_account,
        &ctx.accounts.destination_mint,
        &ctx.accounts.associated_token_program,
        &ctx.accounts.token_program,
        &ctx.accounts.token_2022_program,
        &ctx.accounts.system_program,
        &ctx.remaining_accounts,
    )?;

    Ok(())
}
