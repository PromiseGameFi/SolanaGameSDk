use crate::{constants::*, proxy_swap_process, SwapArgs};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct ProxySwapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
        token::token_program = source_token_program,
    )]
    pub source_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = destination_mint,
        token::token_program = destination_token_program,
    )]
    pub destination_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub source_mint: Box<InterfaceAccount<'info, Mint>>,

    pub destination_mint: Box<InterfaceAccount<'info, Mint>>,

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

pub fn proxy_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, ProxySwapAccounts<'a>>,
    args: SwapArgs,
    order_id: u64,
) -> Result<u64> {
    proxy_swap_process(
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
    )
}
