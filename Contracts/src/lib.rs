//ClanCrowdfund Contract

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
    program::invoke,
    system_instruction,
    clock::Clock,
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Campaign {
    pub creator: Pubkey,
    pub goal: u64,          // In lamports
    pub deadline: i64,      // Unix timestamp
    pub amount_raised: u64,
    pub is_active: bool,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    CreateCampaign { goal: u64, deadline: i64 },
    Contribute { amount: u64 },
    Withdraw,
    Refund,
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = Instruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        Instruction::CreateCampaign { goal, deadline } => {
            create_campaign(program_id, accounts, goal, deadline)
        }
        Instruction::Contribute { amount } => contribute(program_id, accounts, amount),
        Instruction::Withdraw => withdraw(program_id, accounts),
        Instruction::Refund => refund(program_id, accounts),
    }
}

fn create_campaign(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    goal: u64,
    deadline: i64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let creator = next_account_info(accounts_iter)?;
    let campaign_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    if !creator.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let rent = Rent::get()?;
    let space = 8 + 32 + 8 + 8 + 8 + 1; // Discriminator + Campaign struct
    let lamports = rent.minimum_balance(space);

    invoke(
        &system_instruction::create_account(
            creator.key,
            campaign_account.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[creator.clone(), campaign_account.clone(), system_program.clone()],
    )?;

    let campaign = Campaign {
        creator: *creator.key,
        goal,
        deadline,
        amount_raised: 0,
        is_active: true,
    };

    campaign.serialize(&mut &mut campaign_account.data.borrow_mut()[..])?;
    msg!("Campaign created with goal: {} lamports", goal);
    Ok(())
}

fn contribute(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let contributor = next_account_info(accounts_iter)?;
    let campaign_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    if !contributor.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut campaign = Campaign::try_from_slice(&campaign_account.data.borrow())?;
    if !campaign.is_active || campaign.deadline < Clock::get()?.unix_timestamp {
        return Err(ProgramError::InvalidAccountData);
    }

    invoke(
        &system_instruction::transfer(contributor.key, campaign_account.key, amount),
        &[contributor.clone(), campaign_account.clone(), system_program.clone()],
    )?;

    campaign.amount_raised += amount;
    campaign.serialize(&mut &mut campaign_account.data.borrow_mut()[..])?;
    msg!("Contributed {} lamports to campaign", amount);
    Ok(())
}

fn withdraw(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let creator = next_account_info(accounts_iter)?;
    let campaign_account = next_account_info(accounts_iter)?;

    if !creator.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut campaign = Campaign::try_from_slice(&campaign_account.data.borrow())?;
    if campaign.creator != *creator.key {
        return Err(ProgramError::IllegalOwner);
    }

    if campaign.amount_raised < campaign.goal {
        return Err(ProgramError::InsufficientFunds);
    }

    let amount = campaign.amount_raised;
    **campaign_account.lamports.borrow_mut() -= amount;
    **creator.lamports.borrow_mut() += amount;
    campaign.is_active = false;
    campaign.serialize(&mut &mut campaign_account.data.borrow_mut()[..])?;
    msg!("Withdrawn {} lamports from campaign", amount);
    Ok(())
}

fn refund(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let contributor = next_account_info(accounts_iter)?;
    let campaign_account = next_account_info(accounts_iter)?;

    if !contributor.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let campaign = Campaign::try_from_slice(&campaign_account.data.borrow())?;
    if campaign.is_active && campaign.deadline > Clock::get()?.unix_timestamp {
        return Err(ProgramError::InvalidAccountData);
    }

    if campaign.amount_raised >= campaign.goal {
        return Err(ProgramError::InvalidAccountData);
    }

    let refund_amount = campaign.amount_raised;
    **campaign_account.lamports.borrow_mut() -= refund_amount;
    **contributor.lamports.borrow_mut() += refund_amount;
    msg!("Refunded {} lamports to contributor", refund_amount);
    Ok(())
}