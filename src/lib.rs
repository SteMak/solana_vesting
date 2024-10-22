use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

pub mod error;
pub mod helpers;
pub mod instruction;
pub mod pda;
pub mod processor;

entrypoint!(process_instruction);

/// Program entrypoint
pub fn process_instruction<'a, 'b, 'c, 'd>(
    program_id: &'a Pubkey,
    accounts: &'b [AccountInfo<'c>],
    instruction_data: &'d [u8],
) -> ProgramResult {
    processor::process(program_id, accounts, instruction_data)
}

/// Sanity tests
#[cfg(test)]
mod test {
    use super::process_instruction;
    use crate::instruction::VestingInstruction;
    use crate::pda::Vesting;

    use borsh::{BorshDeserialize, BorshSerialize};
    use solana_program::clock::Epoch;
    use solana_sdk::{
        account_info::AccountInfo,
        clock::{Slot, UnixTimestamp},
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        signature::Keypair,
        signer::Signer,
        sysvar::{clock, rent},
    };
    use spl_token::{
        instruction::TokenInstruction,
        state::{Account, Mint},
    };
    use std::mem;

    #[test]
    fn test_sample_workflow() {
        // The test is not accurate as internal calls to spl-token are not performed

        let no_account = Pubkey::default();
        let spl_id = &spl_token::id();
        let program_id = &Pubkey::new_unique();

        // Create user account
        let vester_keypair = Keypair::new();
        let vester_key = vester_keypair.pubkey();
        let vester_bal = &mut 100000000000000;
        let vester = AccountInfo::new(
            &vester_key,
            true,
            true,
            vester_bal,
            &mut [],
            &no_account,
            false,
            Epoch::default(),
        );

        // Create user account
        let seed_keypair = Keypair::new();
        let seed_key = seed_keypair.pubkey();
        let seed_bal = &mut 0;
        let seed = AccountInfo::new(
            &seed_key,
            true,
            false,
            seed_bal,
            &mut [],
            &no_account,
            false,
            Epoch::default(),
        );

        // Create user account
        let claimer_keypair = Keypair::new();
        let claimer_key = claimer_keypair.pubkey();

        // Create rent account
        let rent_key = rent::id();
        #[derive(BorshSerialize, BorshDeserialize)]
        struct MockRent {
            lamports_per_byte_year: u64,
            exemption_threshold: f64,
            burn_percent: u8,
        }
        let rent_data = MockRent {
            lamports_per_byte_year: Rent::default().lamports_per_byte_year,
            exemption_threshold: Rent::default().exemption_threshold,
            burn_percent: Rent::default().burn_percent,
        };
        let rent_encoded = &mut rent_data.try_to_vec().unwrap();
        let rent_bal = &mut Rent::default().minimum_balance(mem::size_of::<MockRent>());
        let rent = AccountInfo::new(
            &rent_key,
            false,
            false,
            rent_bal,
            rent_encoded,
            &no_account,
            false,
            Epoch::default(),
        );

        // Create clock account
        let clock_key = clock::id();
        #[derive(BorshSerialize, BorshDeserialize)]
        struct MockClock {
            pub slot: Slot,
            pub epoch_start_timestamp: UnixTimestamp,
            pub epoch: Epoch,
            pub leader_schedule_epoch: Epoch,
            pub unix_timestamp: UnixTimestamp,
        }
        let clock_data = MockClock {
            slot: 0,
            epoch_start_timestamp: 15200,
            epoch: 0,
            leader_schedule_epoch: 1,
            unix_timestamp: 15400,
        };
        let clock_encoded = &mut clock_data.try_to_vec().unwrap();
        let clock_bal = &mut Rent::default().minimum_balance(mem::size_of::<MockClock>());
        let clock = AccountInfo::new(
            &clock_key,
            false,
            false,
            clock_bal,
            clock_encoded,
            &no_account,
            false,
            Epoch::default(),
        );

        // Create token mint account
        let mint_key = &Pubkey::new_unique();
        let mint_bal = &mut Rent::default().minimum_balance(Mint::LEN);
        let mint_data = &mut [0; Mint::LEN];
        let mint = AccountInfo::new(
            mint_key,
            false,
            true,
            mint_bal,
            mint_data,
            spl_id,
            false,
            Epoch::default(),
        );
        spl_token::processor::Processor::process(
            spl_id,
            &[mint.clone(), rent.clone()],
            &TokenInstruction::InitializeMint {
                mint_authority: no_account,
                freeze_authority: None.into(),
                decimals: 2,
            }
            .pack(),
        )
        .unwrap();

        // Create token wallet account
        // Create vault pda account
        let (distribute_key, _) = Pubkey::find_program_address(
            &["DISTRIBUTE".as_bytes(), &seed_key.as_ref()],
            program_id,
        );
        let distribute_bal = &mut Rent::default().minimum_balance(mem::size_of::<Account>());
        let distribute_data = &mut [0; mem::size_of::<Account>()];
        let distribute = AccountInfo::new(
            &distribute_key,
            false,
            true,
            distribute_bal,
            distribute_data,
            spl_id,
            false,
            Epoch::default(),
        );

        // Create vesting pda account
        let (vesting_key, _) =
            Pubkey::find_program_address(&["VESTING".as_bytes(), &seed_key.as_ref()], program_id);
        let vesting_bal = &mut Rent::default().minimum_balance(mem::size_of::<Vesting>());
        let vesting_data = &mut [0; mem::size_of::<Vesting>()];
        let vesting = AccountInfo::new(
            &vesting_key,
            false,
            true,
            vesting_bal,
            vesting_data,
            program_id,
            false,
            Epoch::default(),
        );

        // Create vault pda account
        let (vault_key, _) =
            Pubkey::find_program_address(&["VAULT".as_bytes(), &seed_key.as_ref()], program_id);
        let vault_bal = &mut Rent::default().minimum_balance(mem::size_of::<Account>());
        let vault_data = &mut [0; mem::size_of::<Account>()];
        let vault = AccountInfo::new(
            &vault_key,
            false,
            true,
            vault_bal,
            vault_data,
            spl_id,
            false,
            Epoch::default(),
        );

        Account {
            amount: 1000000,
            ..Account::default()
        }
        .pack_into_slice(&mut vault.data.borrow_mut()[..]);

        // Create Vesting
        let binding = [
            rent.clone(),
            vester.clone(),
            seed.clone(),
            mint.clone(),
            vesting.clone(),
            vault.clone(),
            distribute.clone(),
        ];
        process_instruction(
            program_id,
            &binding,
            &VestingInstruction::CreateVesting {
                beneficiary: claimer_key,
                amount: 15000,
                start: (clock_data.unix_timestamp - 100) as u64,
                cliff: 0,
                duration: 150,
            }
            .try_to_vec()
            .unwrap(),
        )
        .unwrap();

        // Claim Vesting
        let binding = [
            clock.clone(),
            vesting.clone(),
            vault.clone(),
            distribute.clone(),
        ];
        process_instruction(
            program_id,
            &binding,
            &VestingInstruction::Claim { seed_key }.try_to_vec().unwrap(),
        )
        .unwrap();

        // Check Vesting account state change
        assert_eq!(
            Vesting::try_from_slice(&vesting.data.borrow())
                .unwrap()
                .claimed,
            10000
        );

        // In the test `sol_invoke_signed()`` is not available, so we can't check result balances
    }
}
