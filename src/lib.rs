use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar, program::{invoke, invoke_signed},
};

use spl_token::{self};
use std::slice::Iter;

pub mod pda;

use crate::pda::{State, Vesting};

// user => vesting

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum VestingInstruction {
    Initialize {
        owner: Pubkey,
        token_mint: Pubkey,
    },
    CreateVestingSchedule {
        user: Pubkey,
        amount: u64,
        start_date: u64,
        cliff: u64,
        duration: u64,
    },
    Claim {},
}

//
// Linear, One Token, Flexible for owner
//
// Vesting:
// - Duration
// - Amount
// - Cliff - time
// - Start date
//
//
// Accounts
// - Program
// - State
// - (Single) Vault for the vesting token
// - (Multiple) Vesting data per user
//

pub struct Accounts<'a> {
    pub signer: &'a AccountInfo<'a>,
    pub state: &'a AccountInfo<'a>,
    pub vesting: Option<&'a AccountInfo<'a>>,
    pub wallet: Option<&'a AccountInfo<'a>>,
    pub funds_storage: Option<&'a AccountInfo<'a>>,
}

// Declare and export the program's entrypoint
entrypoint!(process_instruction);

// Program entrypoint's implementation
pub fn process_instruction<'a>(
    program_id: &Pubkey, // Public key of the account the hello world program was loaded into
    infos: &'a [AccountInfo<'a>], // The account to say hello to
    instruction_data: &[u8], // Ignored, all helloworld instructions are hellos
) -> ProgramResult {
    let instruction = VestingInstruction::try_from_slice(instruction_data)?;

    let it: &mut Iter<'a, AccountInfo<'a>> = &mut infos.into_iter();

    let signer = next_account_info(it)?;
    if !signer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let state = next_account_info(it)?;
    State::check_info(state, program_id)?;

    let mut accounts = &mut Accounts {
        state,
        signer,
        vesting: None,
        wallet: None,
        funds_storage: None,
    };

    match instruction {
        VestingInstruction::Initialize { owner, token_mint } => {
            let funds_storage = next_account_info(it)?;
            accounts.funds_storage = Some(funds_storage);

            initialize(program_id, accounts, owner, token_mint)
        }
        VestingInstruction::CreateVestingSchedule {
            user,
            amount,
            start_date,
            cliff,
            duration,
        } => {
            let vesting = next_account_info(it)?;
            Vesting::check_info(vesting, program_id, user)?;

            accounts.vesting = Some(vesting);

            create_vesting_schedule(
                program_id, accounts, user, amount, start_date, cliff, duration,
            )
        }
        VestingInstruction::Claim {} => {
            let vesting = next_account_info(it)?;
            Vesting::check_info(vesting, program_id, *accounts.signer.key)?;

            accounts.vesting = Some(vesting);

            claim(program_id, accounts)
        }
    }
}

fn initialize(
    program_id: &Pubkey,
    accounts: &Accounts,
    owner: Pubkey,
    token_mint: Pubkey,
) -> ProgramResult {
    State::create(program_id, accounts.signer, accounts.state)?;

    let instruction = spl_token::instruction::initialize_account3(
        &spl_token::id(),
        accounts.funds_storage.unwrap().key,
        &token_mint,
        &program_id,
    )?;
    invoke(&instruction, &[accounts.signer.clone()])?;
    State { owner, token_mint }.set_data(accounts.state)?;

    Ok(())
}

//The owner able to create new vesting schedules.
fn create_vesting_schedule<'a, 'b>(
    program_id: &Pubkey,
    accounts: &Accounts<'a>,
    user: Pubkey,
    amount: u64,
    start_date: u64,
    cliff: u64,
    duration: u64,
) -> ProgramResult {
    // Deserialize the state to access owner pubkey
    let state_data = State::get_data(accounts.state)?;
    if state_data.owner != *accounts.signer.key {
        return Err(ProgramError::IllegalOwner);
    }

    // TODO check that if account exist it is not overwritten

    // Create a new vesting schedule
    let new_vesting_schedule = Vesting {
        duration,
        amount,
        cliff,
        start_date,
        claimed: 0,
    };

    Vesting::create(program_id, accounts.signer, accounts.vesting.unwrap(), user)?;

    new_vesting_schedule.set_data(accounts.vesting.unwrap())?;

    let instruction = spl_token::instruction::transfer(
        &spl_token::id(),
        accounts.wallet.unwrap().key,
        accounts.funds_storage.unwrap().key,
        accounts.signer.key,
        &[accounts.signer.key],
        amount
    )?;
    invoke(&instruction, &[accounts.signer.clone()])?;

    Ok(())
}

fn claim<'a>(program_id: &Pubkey, accounts: &Accounts) -> ProgramResult {
    let mut vesting_data = Vesting::get_data(accounts.vesting.unwrap())?;

    let clock = &Clock::get()?;

    let total = calculate_amount(
        vesting_data.start_date,
        vesting_data.cliff,
        vesting_data.duration,
        vesting_data.amount,
        clock.unix_timestamp as u64,
    )?;

    let instruction = spl_token::instruction::transfer(
        &spl_token::id(),
        accounts.funds_storage.unwrap().key,
        accounts.wallet.unwrap().key,
        accounts.signer.key,
        &[accounts.signer.key],
        total - vesting_data.claimed
    )?;
    invoke(&instruction, &[accounts.signer.clone()])?;

    vesting_data.claimed = total;
    vesting_data.set_data(accounts.vesting.unwrap())?;

    Ok(())
}

fn calculate_amount(
    start_date: u64,
    cliff: u64,
    duration: u64,
    vesting_amount: u64,
    now: u64,
) -> Result<u64, ProgramError> {
    if cliff > now {
        return Ok(0);
    }

    let time_passed = now - start_date;
    let calculated_amount = (time_passed / duration) * vesting_amount;

    //Should check how many token claimed before and substract from the calculated_amount
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
