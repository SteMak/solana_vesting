use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use std::{convert::TryInto, slice::Iter};

#[cfg(target_os = "solana")]
use solana_program::sysvar::Sysvar;

use crate::{
    error::CustomError,
    pda::{Vault, Vesting},
};

/// Instruction enum definition
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum VestingInstruction {
    CreateVesting {
        user: Pubkey,
        nonce: u64,

        amount: u64,

        start: u64,
        cliff: u64,
        duration: u64,
    },

    Claim {
        user: Pubkey,
        nonce: u64,
    },
}

/// Structured accounts infos
pub struct Accounts<'a> {
    pub signer: &'a AccountInfo<'a>,
    pub mint: &'a AccountInfo<'a>,

    pub vesting: &'a AccountInfo<'a>,
    pub vault: &'a AccountInfo<'a>,
    pub wallet: &'a AccountInfo<'a>,
}

/// Instructions processor
pub fn process<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    // Decode instruction data
    let instruction = VestingInstruction::try_from_slice(instruction_data)
        .map_err(|x| ProgramError::BorshIoError(x.to_string()))?;

    // Transform account list to iterable object
    let accounts_iter: &mut Iter<'a, AccountInfo<'a>> = &mut accounts.iter();

    // Signer is always needed, validating it
    let signer = next_account_info(accounts_iter)?;
    if !signer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Mint is token identifier, we can't fully validate it
    let mint: &AccountInfo<'a> = next_account_info(accounts_iter)?;
    if *mint.owner != spl_token::id() {
        return Err(ProgramError::Custom(
            CustomError::NotOwnedByTokenProgram.into(),
        ));
    }

    // Someone's wallet, we can't fully validate it
    let wallet: &AccountInfo = next_account_info(accounts_iter)?;
    if *wallet.owner != spl_token::id() {
        return Err(ProgramError::Custom(
            CustomError::NotOwnedByTokenProgram.into(),
        ));
    }

    match instruction {
        VestingInstruction::CreateVesting {
            user,
            nonce,
            amount,
            start,
            cliff,
            duration,
        } => {
            // Vesting PDA, checking seeds compilance, shouldn't be initialized
            let vesting: &AccountInfo = next_account_info(accounts_iter)?;
            Vesting::check_info(vesting, program_id, mint.key, user, nonce)?;

            // Vault PDA, checking seeds compilance, shouldn't be initialized
            let vault: &AccountInfo = next_account_info(accounts_iter)?;
            Vault::check_info(vault, program_id, mint.key, user, nonce)?;

            let accounts = &Accounts {
                signer,
                mint,
                vesting,
                vault,
                wallet,
            };

            // Running logic
            create_vesting(
                program_id, accounts, user, nonce, amount, start, cliff, duration,
            )
        }

        VestingInstruction::Claim { user, nonce } => {
            // Validate signer is vesting owner
            if *signer.key != user {
                return Err(ProgramError::Custom(
                    CustomError::UnauthorizedClaimer.into(),
                ));
            }

            // Vesting PDA, checking seeds compilance, should be initialized
            let vesting: &AccountInfo = next_account_info(accounts_iter)?;
            Vesting::check_info(vesting, program_id, mint.key, user, nonce)?;

            // Vault PDA, checking seeds compilance, should be initialized
            let vault: &AccountInfo = next_account_info(accounts_iter)?;
            Vault::check_info(vault, program_id, mint.key, user, nonce)?;

            let accounts = &Accounts {
                signer,
                mint,
                vesting,
                vault,
                wallet,
            };

            // Running logic
            claim(program_id, accounts, user, nonce)
        }
    }
}

/// Create vesting instruction logic
pub fn create_vesting(
    program_id: &Pubkey,
    accounts: &Accounts<'_>,
    user: Pubkey,
    nonce: u64,
    amount: u64,
    start: u64,
    cliff: u64,
    duration: u64,
) -> ProgramResult {
    // Parameters check
    if cliff > duration {
        return Err(ProgramError::Custom(0));
    }
    if amount == 0 {
        return Err(ProgramError::Custom(0));
    }

    // Create Vesting PDA
    Vesting::create(
        accounts.vesting,
        program_id,
        accounts.signer,
        accounts.mint.key,
        user,
        nonce,
    )?;

    // Set vesting data
    let vesting_schedule = Vesting {
        amount,
        claimed: 0,

        start,
        cliff,
        duration,
    };
    vesting_schedule.set_data(accounts.vesting)?;

    // Create Vault PDA
    Vault::create(
        accounts.vault,
        program_id,
        accounts.signer,
        accounts.mint,
        user,
        nonce,
    )?;

    // Deposit vested funds
    Vault::transfer_in(accounts.vault, accounts.wallet, accounts.signer, amount)?;

    Ok(())
}

/// Claim vesting instruction logic
pub fn claim(program_id: &Pubkey, accounts: &Accounts, user: Pubkey, nonce: u64) -> ProgramResult {
    // Get vesting data
    let mut vesting_data = Vesting::get_data(accounts.vesting)?;

    // Hack to make tests work
    #[cfg(target_os = "solana")]
    let clock = Clock::get()?;
    #[cfg(not(target_os = "solana"))]
    let clock = Clock {
        unix_timestamp: 60 * 60 * 24 * 365,
        ..Clock::default()
    };

    // Get unlocked funds amount
    let total = calculate_amount(
        vesting_data.start,
        vesting_data.cliff,
        vesting_data.duration,
        vesting_data.amount,
        // Causing panic for negative time
        clock.unix_timestamp.try_into().unwrap(),
    )?;

    // Update vesting data
    let distributed = total - vesting_data.claimed;
    vesting_data.claimed = total;
    vesting_data.set_data(accounts.vesting)?;

    // Withdraw distributed funds
    Vault::transfer_out(
        accounts.vault,
        program_id,
        accounts.wallet,
        accounts.mint.key,
        user,
        nonce,
        distributed,
    )?;

    Ok(())
}

/// Get amount unlocked at `now` moment
fn calculate_amount(
    start: u64,
    cliff: u64,
    duration: u64,
    vesting_amount: u64,
    now: u64,
) -> Result<u64, ProgramError> {
    if start + cliff > now {
        return Ok(0);
    }

    let passed = if now - start > duration {
        duration
    } else {
        now - start
    };

    // Due to `u64 * u64 = u128` and `passed / duration <= 1` we have no overflow and best precision
    let calculated_amount = (vesting_amount as u128 * passed as u128 / duration as u128) as u64;

    Ok(calculated_amount)
}
