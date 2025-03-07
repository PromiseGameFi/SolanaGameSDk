use crate::adapters::{
    aldrin, fluxbeam, lifinity, meteora, obric_v2, openbookv2, phoenix, pumpfun, raydium, sanctum, spl_token_swap, stable_swap, whirlpool
};
use crate::error::ErrorCode;
use crate::utils::token::{transfer_token_from_sa_pda, transfer_token_from_user};
use crate::{MAX_HOPS, TOTAL_WEIGHT, ZERO_ADDRESS};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum Dex {
    SplTokenSwap,
    StableSwap,
    Whirlpool,
    MeteoraDynamicpool,
    RaydiumSwap,
    RaydiumStableSwap,
    RaydiumClmmSwap,
    AldrinExchangeV1,
    AldrinExchangeV2,
    LifinityV1,
    LifinityV2,
    RaydiumClmmSwapV2,
    FluxBeam,
    MeteoraDlmm,
    RaydiumCpmmSwap,
    OpenBookV2,
    WhirlpoolV2,
    Phoenix,
    ObricV2,
    SanctumAddLiq,
    SanctumRemoveLiq,
    SanctumNonWsolSwap,
    SanctumWsolSwap,
    PumpfunBuy,
    PumpfunSell,
}
#[derive(Debug)]
pub struct HopAccounts {
    pub last_to_account: Pubkey,
    pub from_account: Pubkey,
    pub to_account: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Route {
    pub dexes: Vec<Dex>,
    pub weights: Vec<u8>,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct SwapArgs {
    pub amount_in: u64,
    pub expect_amount_out: u64,
    pub min_return: u64,
    pub amounts: Vec<u64>,       // 1st level split amount
    pub routes: Vec<Vec<Route>>, // 2nd level split route
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct CommissionSwapArgs {
    pub amount_in: u64,
    pub expect_amount_out: u64,
    pub min_return: u64,
    pub amounts: Vec<u64>,       // 1st level split amount
    pub routes: Vec<Vec<Route>>, // 2nd level split route

    pub commission_rate: u16,       // Commission rate
    pub commission_direction: bool, // Commission direction: true-fromToken, false-toToken
}

#[event]
#[derive(Debug)]
pub struct SwapEvent {
    pub dex: Dex,
    pub amount_in: u64,
    pub amount_out: u64,
}

pub fn proxy_swap_process<'info>(
    payer: &Signer<'info>,
    sa_authority: &UncheckedAccount<'info>,
    source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    source_token_sa: &mut Option<InterfaceAccount<'info, TokenAccount>>,
    destination_token_sa: &mut Option<InterfaceAccount<'info, TokenAccount>>,
    source_mint: &InterfaceAccount<'info, Mint>,
    destination_mint: &InterfaceAccount<'info, Mint>,
    source_token_program: &Interface<'info, TokenInterface>,
    destination_token_program: &Interface<'info, TokenInterface>,
    remaining_accounts: &'info [AccountInfo<'info>],
    args: SwapArgs,
    order_id: u64,
) -> Result<u64> {
    let before_source_balance = source_token_account.amount;
    let before_destination_balance = destination_token_account.amount;
    let min_return = args.min_return;
    msg!(
        "before_source_balance: {}, before_destination_balance: {}, amount_in: {}, expect_amount_out: {}, min_return: {}",
        before_source_balance,
        before_destination_balance,
        args.amount_in,
        args.expect_amount_out,
        min_return
    );

    // 1.Transfer source token to source_token_sa
    let mut source_account = if let Some(source_token_sa) = source_token_sa {
        transfer_token_from_user(
            payer.to_account_info(),
            source_token_account.to_account_info(),
            source_token_sa.to_account_info(),
            source_mint.to_account_info(),
            source_token_program.to_account_info(),
            args.amount_in,
            source_mint.decimals,
        )?;
        source_token_sa.clone()
    } else {
        source_token_account.clone()
    };

    // 2.Smart swap
    let mut destination_account = if let Some(destination_token_sa) = destination_token_sa {
        destination_token_sa.clone()
    } else {
        destination_token_account.clone()
    };
    let amount_out = swap_process(
        &mut source_account,
        &mut destination_account,
        &source_mint,
        &destination_mint,
        remaining_accounts,
        args,
        order_id,
        source_token_sa.is_some(),
    )?;
    msg!("Swap amount_out: {}", amount_out);

    // 3. Transfer destination token to destination_token_account
    if let Some(ref destination_token_sa) = destination_token_sa {
        transfer_token_from_sa_pda(
            sa_authority.to_account_info(),
            destination_token_sa.to_account_info(),
            destination_token_account.to_account_info(),
            destination_mint.to_account_info(),
            destination_token_program.to_account_info(),
            amount_out,
            destination_mint.decimals,
        )?;
    }

    source_token_account.reload()?;
    destination_token_account.reload()?;
    let after_source_balance = source_token_account.amount;
    let after_destination_balance = destination_token_account.amount;
    let source_token_change = before_source_balance
        .checked_sub(after_source_balance)
        .ok_or(ErrorCode::CalculationError)?;
    let destination_token_change = after_destination_balance
        .checked_sub(before_destination_balance)
        .ok_or(ErrorCode::CalculationError)?;
    msg!(
        "after_source_balance: {}, after_destination_balance: {}, source_token_change: {}, destination_token_change: {}",
        after_source_balance,
        after_destination_balance,
        source_token_change,
        destination_token_change
    );

    // CHECK: min_return
    require!(
        destination_token_change >= min_return,
        ErrorCode::MinReturnNotReached
    );
    Ok(amount_out)
}

pub fn swap_process<'info>(
    source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    source_mint: &InterfaceAccount<'info, Mint>,
    destination_mint: &InterfaceAccount<'info, Mint>,
    remaining_accounts: &'info [AccountInfo<'info>],
    args: SwapArgs,
    order_id: u64,
    proxy_swap: bool,
) -> Result<u64> {
    if order_id > 0 {
        msg!("order_id: {}", order_id);
    }
    // Check SwapArgs
    let SwapArgs {
        amount_in,
        min_return,
        expect_amount_out,
        amounts,
        routes,
    } = &args;
    require!(*amount_in > 0, ErrorCode::AmountInMustBeGreaterThanZero);
    require!(*min_return > 0, ErrorCode::MinReturnMustBeGreaterThanZero);
    require!(
        *expect_amount_out >= *min_return,
        ErrorCode::InvalidExpectAmountOut
    );
    require!(
        amounts.len() == routes.len(),
        ErrorCode::AmountsAndRoutesMustHaveTheSameLength
    );

    let total_amounts: u64 = amounts.iter().try_fold(0u64, |acc, &x| {
        acc.checked_add(x).ok_or(ErrorCode::CalculationError)
    })?;
    require!(
        total_amounts == *amount_in,
        ErrorCode::TotalAmountsMustBeEqualToAmountIn
    );
    
    // log source_mint and destination_mint
    source_mint.key().log();
    destination_mint.key().log();

    if proxy_swap {
        source_token_account.reload()?;
        destination_token_account.reload()?;
    }
    let before_source_balance = source_token_account.amount;
    let before_destination_balance = destination_token_account.amount;

    if !proxy_swap {
        msg!(
            "before_source_balance: {}, before_destination_balance: {}, amount_in: {}, expect_amount_out: {}, min_return: {}",
            before_source_balance,
            before_destination_balance,
            amount_in,
            expect_amount_out,
            min_return
        );
    }

    // Swap by Routes
    let mut offset: usize = 0;
    // Level 1 split handling
    for (i, hops) in routes.iter().enumerate() {
        require!(hops.len() <= MAX_HOPS, ErrorCode::TooManyHops);
        let mut amount_in = amounts[i];

        // Multi-hop handling
        let mut last_to_account = ZERO_ADDRESS;
        for (hop, route) in hops.iter().enumerate() {
            let dexes = &route.dexes;
            let weights = &route.weights;
            require!(
                dexes.len() == weights.len(),
                ErrorCode::DexesAndWeightsMustHaveTheSameLength
            );
            let total_weight: u8 = weights.iter().try_fold(0u8, |acc, &x| {
                acc.checked_add(x).ok_or(ErrorCode::CalculationError)
            })?;
            require!(total_weight == TOTAL_WEIGHT, ErrorCode::WeightsMustSumTo100);

            // Level 2 split handling
            let mut hop_accounts = HopAccounts {
                last_to_account,
                from_account: ZERO_ADDRESS,
                to_account: ZERO_ADDRESS,
            };
            let mut amount_out: u64 = 0;
            let mut acc_fork_in: u64 = 0;
            for (index, dex) in dexes.iter().enumerate() {
                // Calculate 2 level split amount
                let fork_amount_in = if index == dexes.len() - 1 {
                    // The last dex, use the remaining amount_in for trading to prevent accumulation
                    amount_in
                        .checked_sub(acc_fork_in)
                        .ok_or(ErrorCode::CalculationError)?
                } else {
                    let temp_amount = amount_in
                        .checked_mul(weights[index] as u64)
                        .ok_or(ErrorCode::CalculationError)?
                        .checked_div(TOTAL_WEIGHT as u64)
                        .ok_or(ErrorCode::CalculationError)?;
                    acc_fork_in = acc_fork_in
                        .checked_add(temp_amount)
                        .ok_or(ErrorCode::CalculationError)?;
                    temp_amount
                };

                // Execute swap
                let fork_amount_out = excute_swap(
                    dex,
                    remaining_accounts,
                    fork_amount_in,
                    &mut offset,
                    &mut hop_accounts,
                    hop,
                    proxy_swap,
                )?;

                // Emit SwapEvent
                let event = SwapEvent {
                    dex: *dex,
                    amount_in: fork_amount_in,
                    amount_out: fork_amount_out,
                };
                emit!(event);
                msg!("{:?}", event);
                hop_accounts.from_account.log();
                hop_accounts.to_account.log();

                amount_out = amount_out
                    .checked_add(fork_amount_out)
                    .ok_or(ErrorCode::CalculationError)?;
            }

            if hop == 0 {
                // CHECK: Verify the first hop's from_token must be consistent with ctx.accounts.source_token_account
                require!(
                    source_token_account.key() == hop_accounts.from_account,
                    ErrorCode::InvalidSourceTokenAccount
                );
            }
            if hop == hops.len() - 1 {
                // CHECK: Verify the last hop's to_account must be consistent with ctx.accounts.destination_token_account
                require!(
                    destination_token_account.key() == hop_accounts.to_account,
                    ErrorCode::InvalidDestinationTokenAccount
                );
            }
            amount_in = amount_out;
            last_to_account = hop_accounts.to_account;
        }
    }

    //source_token_account.reload()?;
    
    // source token account has been closed in pumpfun buy
    if source_token_account.get_lamports() != 0 {
        source_token_account.reload()?;
    }

    destination_token_account.reload()?;
    let after_source_balance = source_token_account.amount;
    let after_destination_balance = destination_token_account.amount;

    let source_token_change = before_source_balance
        .checked_sub(after_source_balance)
        .ok_or(ErrorCode::CalculationError)?;
    let destination_token_change = after_destination_balance
        .checked_sub(before_destination_balance)
        .ok_or(ErrorCode::CalculationError)?;
    if !proxy_swap {
        msg!( 
            "after_source_balance: {}, after_destination_balance: {}, source_token_change: {}, destination_token_change: {}",
            after_source_balance,
            after_destination_balance,
            source_token_change,
            destination_token_change
        );
    }

    // CHECK: min_return
    require!(
        destination_token_change >= *min_return,
        ErrorCode::MinReturnNotReached
    );

    Ok(destination_token_change)
}

fn excute_swap<'a>(
    dex: &Dex,
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
) -> Result<u64> {
    let swap_function = match dex {
        Dex::SplTokenSwap => spl_token_swap::swap,
        Dex::StableSwap => stable_swap::swap,
        Dex::Whirlpool => whirlpool::swap,
        Dex::MeteoraDynamicpool => meteora::swap,
        Dex::RaydiumSwap => raydium::swap,
        Dex::RaydiumStableSwap => raydium::swap_stable,
        Dex::RaydiumClmmSwap => raydium::swap_clmm,
        Dex::RaydiumClmmSwapV2 => raydium::swap_clmm_v2,
        Dex::AldrinExchangeV1 => aldrin::swap_v1,
        Dex::AldrinExchangeV2 => aldrin::swap_v2,
        Dex::LifinityV1 => lifinity::swap_v1,
        Dex::LifinityV2 => lifinity::swap_v2,
        Dex::FluxBeam => fluxbeam::swap,
        Dex::MeteoraDlmm => meteora::swap_dlmm,
        Dex::RaydiumCpmmSwap => raydium::swap_cpmm,
        Dex::OpenBookV2 => openbookv2::place_take_order,
        Dex::WhirlpoolV2 => whirlpool::swap_v2,
        Dex::Phoenix => phoenix::swap,
        Dex::ObricV2 => obric_v2::swap,
        Dex::SanctumAddLiq => sanctum::add_liquidity_handler,
        Dex::SanctumRemoveLiq => sanctum::remove_liquidity_handler,
        Dex::SanctumNonWsolSwap => sanctum::swap_without_wsol_handler,
        Dex::SanctumWsolSwap => sanctum::swap_with_wsol_handler,
        Dex::PumpfunBuy => pumpfun::buy,
        Dex::PumpfunSell => pumpfun::sell,
    };
    swap_function(
        remaining_accounts,
        amount_in,
        offset,
        hop_accounts,
        hop,
        proxy_swap,
    )
}
