use borsh::{BorshDeserialize, BorshSerialize};
use pda::Vault;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use std::slice::Iter;

pub mod pda;
use crate::pda::Vesting;

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

pub struct Accounts<'a> {
    pub signer: &'a AccountInfo<'a>,
    pub mint: &'a AccountInfo<'a>,

    pub vesting: &'a AccountInfo<'a>,
    pub vault: &'a AccountInfo<'a>,
    pub wallet: &'a AccountInfo<'a>,
}

entrypoint!(process_instruction);

pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = VestingInstruction::try_from_slice(instruction_data)
        .map_err(|x| ProgramError::BorshIoError(x.to_string()))?;

    let accounts_iter: &mut Iter<'a, AccountInfo<'a>> = &mut accounts.into_iter();

    let signer = next_account_info(accounts_iter)?;
    if !signer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mint = next_account_info(accounts_iter)?;

    let wallet: &AccountInfo = next_account_info(accounts_iter)?;

    match instruction {
        VestingInstruction::CreateVesting {
            user,
            nonce,
            amount,
            start,
            cliff,
            duration,
        } => {
            let vesting: &AccountInfo = next_account_info(accounts_iter)?;
            Vesting::check_info(vesting, program_id, mint.key, user, nonce)?;

            let vault: &AccountInfo = next_account_info(accounts_iter)?;
            Vault::check_info(vesting, program_id, mint.key, user, nonce)?;

            let accounts = &Accounts {
                signer,
                mint,
                vesting,
                vault,
                wallet,
            };
            create_vesting(
                program_id, accounts, user, nonce, amount, start, cliff, duration,
            )
        }

        VestingInstruction::Claim { user, nonce } => {
            let vesting: &AccountInfo = next_account_info(accounts_iter)?;
            Vesting::check_info(vesting, program_id, mint.key, user, nonce)?;

            let vault: &AccountInfo = next_account_info(accounts_iter)?;
            Vault::check_info(vesting, program_id, mint.key, user, nonce)?;

            let accounts = &Accounts {
                signer,
                mint,
                vesting,
                vault,
                wallet,
            };

            claim(program_id, accounts, user, nonce)
        }
    }
}

fn create_vesting<'a, 'b>(
    program_id: &Pubkey,
    accounts: &Accounts<'a>,
    user: Pubkey,
    nonce: u64,
    amount: u64,
    start: u64,
    cliff: u64,
    duration: u64,
) -> ProgramResult {
    if cliff > duration {
        return Err(ProgramError::Custom(0));
    }
    if amount == 0 {
        return Err(ProgramError::Custom(0));
    }

    let new_vesting_schedule = Vesting {
        amount,
        claimed: 0,

        start,
        cliff,
        duration,
    };

    Vesting::create(
        accounts.vesting,
        program_id,
        accounts.signer,
        accounts.mint.key,
        user,
        nonce,
    )?;

    new_vesting_schedule.set_data(accounts.vesting)?;

    Vault::create(
        accounts.vault,
        program_id,
        accounts.signer,
        accounts.mint,
        user,
        nonce,
    )?;

    Vault::transfer_in(accounts.vault, accounts.wallet, accounts.signer, amount)?;

    Ok(())
}

fn claim<'a>(program_id: &Pubkey, accounts: &Accounts, user: Pubkey, nonce: u64) -> ProgramResult {
    let mut vesting_data = Vesting::get_data(accounts.vesting)?;

    let clock = &Clock::get()?;

    let total = calculate_amount(
        vesting_data.start,
        vesting_data.cliff,
        vesting_data.duration,
        vesting_data.amount,
        clock.unix_timestamp as u64,
    )?;

    Vault::transfer_out(
        accounts.vault,
        program_id,
        accounts.wallet,
        accounts.mint.key,
        user,
        nonce,
        total - vesting_data.claimed,
    )?;

    vesting_data.claimed = total;
    vesting_data.set_data(accounts.vesting)?;

    Ok(())
}

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

    let time_passed = now - start;
    let calculated_amount = vesting_amount * time_passed / duration;

    return Ok(calculated_amount);
}

// // Sanity tests
// #[cfg(test)]
// mod test {
//     use super::*;
//     use solana_program::{clock::Epoch, instruction::AccountMeta};
//     use solana_program_test::*;
//     use solana_sdk::{account::Account, account_info::AccountInfo, pubkey::Pubkey, rent::Rent};
//     use std::str::FromStr;

//     #[derive(BorshSerialize, BorshDeserialize)]
//     struct MockRent {
//         lamports_per_byte_year: u64,
//         exemption_threshold: f64,
//         burn_percent: u8,
//     }

//     #[test]
//     fn test_sanity() {
//         let program_id = Pubkey::default();
//         let key = rent::id();
//         let mut lamports = 0;
//         let mock_rent = MockRent {
//             lamports_per_byte_year: 3, // example value
//             exemption_threshold: 2.0,  // example value
//             burn_percent: 10,          // example value
//         };
//         let serialized_rent = mock_rent.try_to_vec().unwrap();
//         let mut data = serialized_rent; // Now data contains a serialized Rent object
//         let owner = Pubkey::default();
//         let account = AccountInfo::new(
//             &key,
//             false,
//             false,
//             &mut lamports,
//             &mut data,
//             &owner,
//             true,
//             Epoch::default(),
//         );

//         let instruction_data: Vec<u8> = VestingInstruction::Initialize {
//             owner: Pubkey::default(),
//         }
//         .try_to_vec()
//         .unwrap();

//         let accounts: Vec<AccountInfo<'_>> = vec![
//             account.clone(),
//             account.clone(),
//             account.clone(),
//             account.clone(),
//             account.clone(),
//         ];

//         process_instruction(&program_id, &accounts, &instruction_data).unwrap();

//         // assert_eq!(
//         //     GreetingAccount::try_from_slice(&accounts[0].data.borrow())
//         //         .unwrap()
//         //         .counter,
//         //     0
//         // );
//     }

//     #[test]
//     fn test_create_vesting_schedule() {
//         let program_id = Pubkey::new_unique();
//         let owner_key = Pubkey::new_unique();
//         let user_key = Pubkey::new_unique();
//         let state_key = Pubkey::new_unique();
//         let vesting_account_key = Pubkey::new_unique();
//         let mut lamports = Rent::default().minimum_balance(std::mem::size_of::<State>());
//         let state = State {
//             owner: owner_key,
//             token: Pubkey::default(), // Assuming a default token for simplicity
//         };

//         let mut lamports_for_owner = 0;
//         let mut lamports_for_vesting =
//             Rent::default().minimum_balance(std::mem::size_of::<Vesting>());
//         let mut lamports_for_state = Rent::default().minimum_balance(std::mem::size_of::<State>());
//         let mut lamports_for_rent = 0;

//         let mut state_account = Account::new(0, 0, &program_id);

//         let mut vesting_account =
//             Account::new(lamports, std::mem::size_of::<Vesting>(), &program_id);

//         let rent_key = rent::id();
//         let mock_rent = MockRent {
//             lamports_per_byte_year: 3, // example value
//             exemption_threshold: 2.0,  // example value
//             burn_percent: 10,          // example value
//         };
//         let serialized_rent = mock_rent.try_to_vec().unwrap();
//         let mut data = serialized_rent; // Now data contains a serialized Rent object

//         let system_rent_clock_account_info = AccountInfo::new(
//             &rent_key,
//             false,
//             false,
//             &mut lamports_for_rent,
//             &mut data,
//             &owner_key,
//             true,
//             Epoch::default(),
//         );

//         let owner_account_info = AccountInfo::new(
//             &owner_key,
//             true,  // is_signer
//             false, // is_writable
//             &mut lamports_for_owner,
//             &mut [], // data
//             &program_id,
//             false, // executable
//             Epoch::default(),
//         );

//         let vesting_account_info = AccountInfo::new(
//             &vesting_account_key,
//             false, // is_signer
//             true,  // is_writable
//             &mut lamports_for_vesting,
//             &mut vesting_account.data,
//             &program_id,
//             false, // executable
//             Epoch::default(),
//         );

//         state.serialize(&mut state_account.data).unwrap();
//         let state_date_len = state_account.data.len();
//         let state_account_info = AccountInfo::new(
//             &state_key,
//             false, // is_signer
//             true,  // is_writable
//             &mut lamports_for_state,
//             &mut state_account.data,
//             &program_id,
//             false, // executable
//             Epoch::default(),
//         );

//         assert_eq!(state_date_len, std::mem::size_of::<State>());

//         let accounts = vec![
//             state_account_info.clone(),
//             vesting_account_info.clone(),
//             owner_account_info.clone(),
//         ];

//         let amount = 1000;
//         let start_date = 1234567890;
//         let cliff = 3600; // 1 hour
//         let duration = 86400; // 1 day

//         // Prepare instruction data for create_vesting_schedule
//         let instruction_data = VestingInstruction::CreateVestingSchedule {
//             user: user_key,
//             amount,
//             start_date,
//             cliff,
//             duration,
//         }
//         .try_to_vec()
//         .unwrap();

//         // Simulate the process_instruction call
//         process_instruction(&program_id, &accounts, &instruction_data).unwrap();

//         let vesting_data = Vesting::try_from_slice(&vesting_account.data).unwrap();
//         assert_eq!(vesting_data.amount, amount);
//         assert_eq!(vesting_data.start_date, start_date);
//         assert_eq!(vesting_data.cliff, cliff);
//         assert_eq!(vesting_data.duration, duration);
//     }
// }
