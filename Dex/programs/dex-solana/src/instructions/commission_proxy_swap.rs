use crate::constants::*;
use crate::error::ErrorCode;
use crate::utils::token::{transfer_sol_from_user, transfer_token_from_user};
use crate::{proxy_swap_process, SwapArgs, COMMISSION_DENOMINATOR, COMMISSION_RATE_LIMIT};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct CommissionSOLProxySwapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
    )]
    pub source_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = destination_mint,
    )]
    pub destination_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub source_mint: Box<InterfaceAccount<'info, Mint>>,

    pub destination_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub commission_account: SystemAccount<'info>,

    /// CHECK: sa_authority
    #[account(
        seeds = [
            SEED_SA,
        ],
        bump = BUMP_SA,
    )]
    pub sa_authority: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = source_mint,
        associated_token::authority = sa_authority,
        associated_token::token_program = source_token_program,
    )]
    pub source_token_sa: Option<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = destination_mint,
        associated_token::authority = sa_authority,
        associated_token::token_program = destination_token_program,
    )]
    pub destination_token_sa: Option<InterfaceAccount<'info, TokenAccount>>,

    pub source_token_program: Interface<'info, TokenInterface>,

    pub destination_token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

pub fn commission_sol_proxy_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSOLProxySwapAccounts<'a>>,
    args: SwapArgs,
    commission_rate: u16,
    commission_direction: bool,
    order_id: u64,
) -> Result<u64> {
    // Check commission_rate
    require!(
        commission_rate > 0 && commission_rate <= COMMISSION_RATE_LIMIT,
        ErrorCode::InvalidCommissionRate
    );

    let mut commission_amount: u64 = 0;
    if commission_direction {
        // Commission direction: true-fromToken
        require!(
            ctx.accounts.source_mint.key() == wsol_program::id(),
            ErrorCode::InvalidCommissionTokenAccount
        );

        // Commission for fromToken
        commission_amount = args
            .amount_in
            .checked_mul(commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?
            .checked_div(COMMISSION_DENOMINATOR - commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?;
    } else {
        // Commission direction: false-toToken
        require!(
            ctx.accounts.destination_mint.key() == wsol_program::id(),
            ErrorCode::InvalidCommissionTokenAccount
        );
    }

    // Proxy Swap
    let amount_out = proxy_swap_process(
        &ctx.accounts.payer,
        &ctx.accounts.sa_authority,
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &mut ctx.accounts.source_token_sa,
        &mut ctx.accounts.destination_token_sa,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        &ctx.accounts.source_token_program,
        &ctx.accounts.destination_token_program,
        ctx.remaining_accounts,
        args,
        order_id,
    )?;

    // Commission for toToken
    if !commission_direction {
        commission_amount = amount_out
            .checked_mul(commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?
            .checked_div(COMMISSION_DENOMINATOR)
            .ok_or(ErrorCode::CalculationError)?;
    }

    // Transfer commission_amount
    transfer_sol_from_user(
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.commission_account.to_account_info(),
        commission_amount,
    )?;
    msg!(
        "commission_direction: {:?}, commission_amount: {:?}",
        commission_direction,
        commission_amount
    );
    Ok(amount_out)
}

#[derive(Accounts)]
pub struct CommissionSPLProxySwapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
    )]
    pub source_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = destination_mint,
    )]
    pub destination_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub source_mint: Box<InterfaceAccount<'info, Mint>>,

    pub destination_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub commission_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: sa_authority
    #[account(
        seeds = [
            SEED_SA,
        ],
        bump = BUMP_SA,
    )]
    pub sa_authority: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = source_mint,
        associated_token::authority = sa_authority,
        associated_token::token_program = source_token_program,
    )]
    pub source_token_sa: Option<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = destination_mint,
        associated_token::authority = sa_authority,
        associated_token::token_program = destination_token_program,
    )]
    pub destination_token_sa: Option<InterfaceAccount<'info, TokenAccount>>,

    pub source_token_program: Interface<'info, TokenInterface>,

    pub destination_token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

pub fn commission_spl_proxy_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSPLProxySwapAccounts<'a>>,
    args: SwapArgs,
    commission_rate: u16,
    commission_direction: bool,
    order_id: u64,
) -> Result<u64> {
    // Check commission_rate
    require!(
        commission_rate > 0 && commission_rate <= COMMISSION_RATE_LIMIT,
        ErrorCode::InvalidCommissionRate
    );

    let mut commission_amount: u64 = 0;
    if commission_direction {
        // Commission direction: true-fromToken
        require!(
            ctx.accounts.commission_token_account.mint == ctx.accounts.source_mint.key(),
            ErrorCode::InvalidCommissionTokenAccount
        );

        // Commission for fromToken
        commission_amount = args
            .amount_in
            .checked_mul(commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?
            .checked_div(COMMISSION_DENOMINATOR - commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?;
    } else {
        // Commission direction: false-toToken
        require!(
            ctx.accounts.commission_token_account.mint == ctx.accounts.destination_mint.key(),
            ErrorCode::InvalidCommissionTokenAccount
        );
    }

    // Proxy Swap
    let amount_out = proxy_swap_process(
        &ctx.accounts.payer,
        &ctx.accounts.sa_authority,
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &mut ctx.accounts.source_token_sa,
        &mut ctx.accounts.destination_token_sa,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        &ctx.accounts.source_token_program,
        &ctx.accounts.destination_token_program,
        ctx.remaining_accounts,
        args,
        order_id,
    )?;

    // Transfer commission_amount
    if commission_direction {
        // Commission for fromToken
        transfer_token_from_user(
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.source_token_account.to_account_info(),
            ctx.accounts.commission_token_account.to_account_info(),
            ctx.accounts.source_mint.to_account_info(),
            ctx.accounts.source_token_program.to_account_info(),
            commission_amount,
            ctx.accounts.source_mint.decimals,
        )?;
    } else {
        // Commission for toToken
        commission_amount = amount_out
            .checked_mul(commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?
            .checked_div(COMMISSION_DENOMINATOR)
            .ok_or(ErrorCode::CalculationError)?;

        transfer_token_from_user(
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.destination_token_account.to_account_info(),
            ctx.accounts.commission_token_account.to_account_info(),
            ctx.accounts.destination_mint.to_account_info(),
            ctx.accounts.destination_token_program.to_account_info(),
            commission_amount,
            ctx.accounts.destination_mint.decimals,
        )?;
    }
    msg!(
        "commission_direction: {:?}, commission_amount: {:?}",
        commission_direction,
        commission_amount
    );
    Ok(amount_out)
}
