use crate::error::ErrorCode;
use crate::{BUMP_SA, SEED_SA};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke, system_instruction::transfer};
use anchor_spl::token_2022::{self};

pub fn transfer_sol_from_user<'a>(
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    lamports: u64,
) -> Result<()> {
    if lamports == 0 {
        return Ok(());
    }
    let ix = transfer(from.key, to.key, lamports);
    let res = invoke(&ix, &[from, to]);
    require!(res.is_ok(), ErrorCode::TransferSolFailed);
    Ok(())
}

pub fn transfer_token_from_user<'a>(
    authority: AccountInfo<'a>,
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    amount: u64,
    mint_decimals: u8,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let res = token_2022::transfer_checked(
        CpiContext::new(
            token_program.to_account_info(),
            token_2022::TransferChecked {
                from,
                to,
                authority,
                mint,
            },
        ),
        amount,
        mint_decimals,
    );
    require!(res.is_ok(), ErrorCode::TransferTokenFailed);
    Ok(())
}

pub fn transfer_token_from_sa_pda<'a>(
    authority: AccountInfo<'a>,
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    amount: u64,
    mint_decimals: u8,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let res = token_2022::transfer_checked(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            token_2022::TransferChecked {
                from,
                to,
                authority,
                mint,
            },
            &[&[SEED_SA, &[BUMP_SA]]],
        ),
        amount,
        mint_decimals,
    );
    require!(res.is_ok(), ErrorCode::TransferTokenFailed);
    Ok(())
}
