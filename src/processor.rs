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
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
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
            let rent = &Rent::from_account_info(next_account_info(accounts_iter)?)?;

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

            // Load mint account
            let mint = next_account_info(accounts_iter)?;

            // Prepare PDAs and validate pubkeys
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
            let accounts = &mut CreateVestingAccounts {
                rent,
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

        VestingInstruction::Claim { seed_key } => {
            // Validating clock sysvar
            let clock = &Clock::from_account_info(next_account_info(accounts_iter)?)?;

            // Prepare PDAs and validate pubkeys
            let vesting =
                &mut PDA::<Vesting>::new(program_id, next_account_info(accounts_iter)?, &seed_key)?;
            let vault =
                &mut PDA::<Vault>::new(program_id, next_account_info(accounts_iter)?, &seed_key)?;
            let distribute = &mut PDA::<Distribute>::new(
                program_id,
                next_account_info(accounts_iter)?,
                &seed_key,
            )?;

            // Prepare accounts
            let accounts = &mut ClaimAccounts {
                clock,
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
    accounts: &mut CreateVestingAccounts,
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
    accounts.vesting.data = Vesting {
        beneficiary,
        creator: *accounts.signer.key,
        mint: *accounts.mint.key,
        seed_key: *accounts.seed.key,

        amount,
        claimed: 0,

        start,
        cliff,
        duration,
    };
    accounts.vesting.write()?;

    Ok(())
}

/// Claim vesting instruction logic
pub fn claim(accounts: &mut ClaimAccounts) -> ProgramResult {
    // Get unlocked funds amount
    let total = calculate_amount(
        accounts.vesting.data.start,
        accounts.vesting.data.cliff,
        accounts.vesting.data.duration,
        accounts.vesting.data.amount,
        // Causing panic for negative time
        accounts.clock.unix_timestamp.try_into().unwrap(),
    );

    let distribute = (total - accounts.vesting.data.claimed).min(accounts.vault.data.amount);

    // Update vesting data
    accounts.vesting.data.claimed += distribute;
    accounts.vesting.write()?;

    // Withdraw distributed funds
    if distribute > 0 {
        accounts
            .vault
            .transfer_out(accounts.distribute.info, distribute)?;
    }

    Ok(())
}

/// Get amount unlocked at `now` moment
fn calculate_amount(start: u64, cliff: u64, duration: u64, amount: u64, now: u64) -> u64 {
    if start + cliff > now {
        return 0;
    }

    if now - start >= duration {
        // Free any funds left in Vault
        return u64::MAX;
    }

    // Due to `u64 * u64 = u128` and `(now - start) / duration < 1` we have no overflow and best precision
    (amount as u128 * (now - start) as u128 / duration as u128) as u64
}

/// Sanity tests
#[cfg(test)]
mod test {
    use solana_sdk::{account_info::AccountInfo, clock::Epoch, pubkey::Pubkey, rent::Rent};

    use crate::pda::{Distribute, Vault, Vesting, PDA};

    use super::{calculate_amount, create_vesting, CreateVestingAccounts};

    #[test]
    fn test_calculate_amount() {
        assert_eq!(calculate_amount(0, 0, 0, 0, 500), u64::MAX);
        assert_eq!(calculate_amount(1000, 20, 100, 1000, 500), 0);
        assert_eq!(calculate_amount(1000, 20, 100, 1000, 1000), 0);
        assert_eq!(calculate_amount(1000, 20, 100, 1000, 1010), 0);
        assert_eq!(calculate_amount(1000, 20, 100, 1000, 1019), 0);
        assert_eq!(calculate_amount(1000, 20, 100, 1000, 1020), 200);
        assert_eq!(calculate_amount(1000, 20, 100, 1000, 1090), 900);
        assert_eq!(calculate_amount(1000, 20, 100, 1000, 1099), 990);
        assert_eq!(calculate_amount(1000, 20, 100, 1000, 1100), u64::MAX);
        assert_eq!(calculate_amount(1000, 20, 100, 1000, 1200), u64::MAX);
    }

    #[test]
    fn test_create_vesting_revert() {
        let no_account = Pubkey::default();
        let lamports = &mut 0;

        let dummy_account = AccountInfo::new(
            &no_account,
            false,
            false,
            lamports,
            &mut [],
            &no_account,
            false,
            Epoch::default(),
        );
        let vesting_accounts = &mut CreateVestingAccounts {
            rent: &Rent::default(),
            signer: &dummy_account,
            mint: &dummy_account,
            seed: &dummy_account,
            vesting: &mut PDA {
                data: Vesting::default(),
                info: &dummy_account,
                program_id: &no_account,
                seeds: vec![],
            },
            vault: &mut PDA {
                data: Vault::default(),
                info: &dummy_account,
                program_id: &no_account,
                seeds: vec![],
            },
            distribute: &mut PDA {
                data: Distribute::default(),
                info: &dummy_account,
                program_id: &no_account,
                seeds: vec![],
            },
        };

        create_vesting(vesting_accounts, Pubkey::new_unique(), 10, 15, 40, 30).unwrap_err();
        create_vesting(vesting_accounts, Pubkey::new_unique(), 10, u64::MAX, 20, 30).unwrap_err();
        create_vesting(vesting_accounts, Pubkey::new_unique(), 0, 15, 20, 30).unwrap_err();
    }
}
