use crate::error::ErrorCode;
use crate::utils::token::{transfer_sol_from_user, transfer_token_from_user};
use crate::{
    swap_process, wsol_program, CommissionSwapArgs, SwapArgs, COMMISSION_DENOMINATOR,
    COMMISSION_RATE_LIMIT,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct CommissionSOLAccounts<'info> {
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

    pub destination_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub commission_account: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn commission_sol_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSOLAccounts<'a>>,
    args: CommissionSwapArgs,
    order_id: u64,
) -> Result<u64> {
    // CHECK: CommissionSwapArgs
    require!(
        args.commission_rate > 0 && args.commission_rate <= COMMISSION_RATE_LIMIT,
        ErrorCode::InvalidCommissionRate
    );

    let mut commission_amount: u64 = 0;
    if args.commission_direction {
        // Commission direction: true-fromToken
        require!(
            ctx.accounts.source_mint.key() == wsol_program::id(),
            ErrorCode::InvalidCommissionTokenAccount
        );

        // Commission for fromToken
        commission_amount = args
            .amount_in
            .checked_mul(args.commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?
            .checked_div(COMMISSION_DENOMINATOR - args.commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?;
    } else {
        // Commission direction: false-toToken
        require!(
            ctx.accounts.destination_mint.key() == wsol_program::id(),
            ErrorCode::InvalidCommissionTokenAccount
        );
    }

    let swap_args = SwapArgs {
        amount_in: args.amount_in,
        expect_amount_out: args.expect_amount_out,
        min_return: args.min_return,
        amounts: args.amounts,
        routes: args.routes,
    };
    let amount_out = swap_process(
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        ctx.remaining_accounts,
        swap_args,
        order_id,
        false,
    )?;

    // Commission for toToken
    if !args.commission_direction {
        commission_amount = amount_out
            .checked_mul(args.commission_rate as u64)
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
        args.commission_direction,
        commission_amount
    );
    Ok(amount_out)
}

#[derive(Accounts)]
pub struct CommissionSPLAccounts<'info> {
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

    pub destination_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::token_program = token_program,
    )]
    pub commission_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn commission_spl_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSPLAccounts<'a>>,
    args: CommissionSwapArgs,
    order_id: u64,
) -> Result<u64> {
    // CHECK: CommissionSwapArgs
    require!(
        args.commission_rate > 0 && args.commission_rate <= COMMISSION_RATE_LIMIT,
        ErrorCode::InvalidCommissionRate
    );

    let mut commission_amount: u64 = 0;
    if args.commission_direction {
        // Commission direction: true-fromToken
        require!(
            ctx.accounts.commission_token_account.mint == ctx.accounts.source_mint.key(),
            ErrorCode::InvalidCommissionTokenAccount
        );

        // Commission for fromToken
        commission_amount = args
            .amount_in
            .checked_mul(args.commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?
            .checked_div(COMMISSION_DENOMINATOR - args.commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?;
    } else {
        // Commission direction: false-toToken
        require!(
            ctx.accounts.commission_token_account.mint == ctx.accounts.destination_mint.key(),
            ErrorCode::InvalidCommissionTokenAccount
        );
    }

    let swap_args = SwapArgs {
        amount_in: args.amount_in,
        expect_amount_out: args.expect_amount_out,
        min_return: args.min_return,
        amounts: args.amounts,
        routes: args.routes,
    };
    let amount_out = swap_process(
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        ctx.remaining_accounts,
        swap_args,
        order_id,
        false,
    )?;

    // Transfer commission_amount
    if args.commission_direction {
        // Commission for fromToken
        transfer_token_from_user(
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.source_token_account.to_account_info(),
            ctx.accounts.commission_token_account.to_account_info(),
            ctx.accounts.source_mint.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            commission_amount,
            ctx.accounts.source_mint.decimals,
        )?;
    } else {
        // Commission for toToken
        commission_amount = amount_out
            .checked_mul(args.commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?
            .checked_div(COMMISSION_DENOMINATOR)
            .ok_or(ErrorCode::CalculationError)?;

        transfer_token_from_user(
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.destination_token_account.to_account_info(),
            ctx.accounts.commission_token_account.to_account_info(),
            ctx.accounts.destination_mint.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            commission_amount,
            ctx.accounts.destination_mint.decimals,
        )?;
    }
    msg!(
        "commission_direction: {:?}, commission_amount: {:?}",
        args.commission_direction,
        commission_amount
    );
    Ok(amount_out)
}
