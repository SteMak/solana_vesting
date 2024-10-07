use std::convert::TryInto;

use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
};

use crate::{
    error::CustomError,
    instruction::{ClaimAccounts, CreateVestingAccounts, VestingInstruction},
    pda::{Distribute, PDAMethods, Vault, Vesting, PDA},
};

/// Instructions processor
pub fn process<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    // Decode instruction data
    let instruction = VestingInstruction::try_from_slice(instruction_data)
        .map_err(|x| ProgramError::BorshIoError(x.to_string()))?;

    // Transform account list to iterable object
    let accounts_iter = &mut accounts.iter();

    // Chose instruction to process from enum
    match instruction {
        VestingInstruction::CreateVesting {
            beneficiary,
            amount,
            start,
            cliff,
            duration,
        } => {
            // Validating rent sysvar
            let rent = Rent::from_account_info(&accounts[0])?;
            let rent_ = &rent;

            // Validating signer
            let signer = next_account_info(accounts_iter)?;
            if !signer.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }

            // Validating seed signer
            let seed = next_account_info(accounts_iter)?;
            if !seed.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }

            // Mint is token identifier, we can't fully validate it
            let mint = next_account_info(accounts_iter)?;
            if *mint.owner != spl_token::id() {
                return Err(ProgramError::Custom(
                    CustomError::NotOwnedByTokenProgram.into(),
                ));
            }

            let vesting =
                &mut PDA::<Vesting>::new(program_id, next_account_info(accounts_iter)?, seed.key)?;
            let vault =
                &mut PDA::<Vault>::new(program_id, next_account_info(accounts_iter)?, seed.key)?;
            let distribute = &mut PDA::<Distribute>::new(
                program_id,
                next_account_info(accounts_iter)?,
                seed.key,
            )?;

            // Prepare accounts
            let accounts = CreateVestingAccounts {
                rent: rent_,
                signer,
                seed,
                mint,
                vesting,
                vault,
                distribute,
            };

            // Running logic
            create_vesting(accounts, beneficiary, amount, start, cliff, duration)
        }

        VestingInstruction::Claim {} => {
            // Validating clock sysvar
            let clock = &Clock::from_account_info(next_account_info(accounts_iter)?)?;

            // Validating seed signer
            let seed = next_account_info(accounts_iter)?;
            if !seed.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }

            let vesting =
                &mut PDA::<Vesting>::new(program_id, next_account_info(accounts_iter)?, seed.key)?;
            let vault = &mut PDA::<'_, Vault>::new(
                program_id,
                next_account_info(accounts_iter)?,
                seed.key,
            )?;
            let distribute = &mut PDA::<'_, Distribute>::new(
                program_id,
                next_account_info(accounts_iter)?,
                seed.key,
            )?;

            // Prepare accounts
            let accounts = ClaimAccounts {
                clock,
                seed,
                vesting,
                vault,
                distribute,
            };

            // Running logic
            claim(accounts)
        }
    }
}

/// Create vesting instruction logic
pub fn create_vesting(
    accounts: CreateVestingAccounts,
    beneficiary: Pubkey,
    amount: u64,
    start: u64,
    cliff: u64,
    duration: u64,
) -> ProgramResult {
    // Prevent overflow
    if start.overflowing_add(cliff).1 {
        return Err(ProgramError::Custom(CustomError::StartCliffOverflow.into()));
    }
    // Parameters check
    if cliff > duration {
        return Err(ProgramError::Custom(CustomError::CliffOverDuration.into()));
    }
    if amount == 0 {
        return Err(ProgramError::Custom(CustomError::ZeroAmount.into()));
    }

    // Create Vesting PDA
    accounts.vesting.create(accounts.rent, accounts.signer)?;
    accounts
        .vault
        .create(accounts.rent, accounts.signer, accounts.mint)?;
    accounts
        .distribute
        .create(accounts.rent, accounts.signer, accounts.mint, &beneficiary)?;

    // Set vesting data
    accounts.vesting.write(Vesting {
        beneficiary,
        creator: *accounts.signer.key,
        mint: *accounts.mint.key,
        seed_key: *accounts.seed.key,

        amount,
        claimed: 0,

        start,
        cliff,
        duration,
    })?;

    Ok(())
}

/// Claim vesting instruction logic
pub fn claim(accounts: ClaimAccounts) -> ProgramResult {
    // Get unlocked funds amount
    let total = calculate_amount(
        accounts.vesting.data.start,
        accounts.vesting.data.cliff,
        accounts.vesting.data.duration,
        accounts.vesting.data.amount,
        // Causing panic for negative time
        accounts.clock.unix_timestamp.try_into().unwrap(),
    );

    let mut distribute = total - accounts.vesting.data.claimed;
    if accounts.vault.data.amount < distribute {
        distribute = accounts.vault.data.amount;
    }

    // Update vesting data
    let mut vesting = accounts.vesting.data.clone();

    vesting.claimed += distribute;
    accounts.vesting.write(vesting)?;

    // Withdraw distributed funds
    accounts
        .vault
        .transfer_out(accounts.distribute.info, distribute)?;

    Ok(())
}

/// Get amount unlocked at `now` moment
fn calculate_amount(start: u64, cliff: u64, duration: u64, amount: u64, now: u64) -> u64 {
    if start + cliff > now {
        return 0;
    }

    if now - start >= duration {
        return amount;
    }

    // Due to `u64 * u64 = u128` and `(now - start) / duration < 1` we have no overflow and best precision
    (amount as u128 * (now - start) as u128 / duration as u128) as u64
}

// /// Sanity tests
// #[cfg(test)]
// mod test {
//     use solana_sdk::{account_info::AccountInfo, clock::Epoch, pubkey::Pubkey, rent::Rent};

//     use super::{calculate_amount, create_vesting, CreateVestingAccounts};

//     #[test]
//     fn test_calculate_amount() {
//         assert!(calculate_amount(0, 0, 0, 0, 500) == 0);
//         assert!(calculate_amount(1000, 20, 100, 1000, 500) == 0);
//         assert!(calculate_amount(1000, 20, 100, 1000, 1000) == 0);
//         assert!(calculate_amount(1000, 20, 100, 1000, 1010) == 0);
//         assert!(calculate_amount(1000, 20, 100, 1000, 1019) == 0);
//         assert!(calculate_amount(1000, 20, 100, 1000, 1020) == 200);
//         assert!(calculate_amount(1000, 20, 100, 1000, 1090) == 900);
//         assert!(calculate_amount(1000, 20, 100, 1000, 1099) == 990);
//         assert!(calculate_amount(1000, 20, 100, 1000, 1100) == 1000);
//         assert!(calculate_amount(1000, 20, 100, 1000, 1200) == 1000);
//     }

//     #[test]
//     fn test_create_vesting_revert() {
//         let no_account = Pubkey::default();
//         let lamports = &mut 0;
//         let dummy_account = AccountInfo::new(
//             &no_account,
//             false,
//             false,
//             lamports,
//             &mut [],
//             &no_account,
//             false,
//             Epoch::default(),
//         );
//         let vesting_accounts = CreateVestingAccounts {
//             mint: &dummy_account,
//             signer: &dummy_account,
//             vault: &dummy_account,
//             vesting: &dummy_account,
//             wallet: &dummy_account,
//             rent: &Rent::default(),
//         };

//         create_vesting(
//             &Pubkey::new_unique(),
//             &vesting_accounts,
//             Pubkey::new_unique(),
//             3,
//             1000,
//             1000,
//             120,
//             100,
//         )
//         .unwrap_err();

//         create_vesting(
//             &Pubkey::new_unique(),
//             &vesting_accounts,
//             Pubkey::new_unique(),
//             3,
//             0,
//             1000,
//             20,
//             100,
//         )
//         .unwrap_err();

//         create_vesting(
//             &Pubkey::new_unique(),
//             &vesting_accounts,
//             Pubkey::new_unique(),
//             3,
//             1000,
//             u64::MAX - 10,
//             20,
//             100,
//         )
//         .unwrap_err();
//     }
// }
