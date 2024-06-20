use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

pub mod error;
pub mod helpers;
pub mod pda;
pub mod processor;

entrypoint!(process_instruction);

/// Program entrypoint
pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process(program_id, accounts, instruction_data)
}

/// Sanity tests
#[cfg(test)]
mod test {
    use crate::pda::*;
    use crate::processor::*;

    use borsh::{BorshDeserialize, BorshSerialize};
    use solana_program::clock::Epoch;
    use solana_sdk::{
        account_info::AccountInfo, clock::Clock, program_pack::Pack, pubkey::Pubkey, rent::Rent,
        signature::Keypair, signer::Signer, sysvar::rent,
    };
    use spl_token::{
        instruction::TokenInstruction,
        state::{Account, Mint},
    };
    use std::mem;

    #[test]
    fn sample_workflow() {
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
        let claimer_keypair = Keypair::new();
        let claimer_key = claimer_keypair.pubkey();
        let claimer_bal = &mut 100000000000000;
        let claimer = AccountInfo::new(
            &claimer_key,
            true,
            true,
            claimer_bal,
            &mut [],
            &no_account,
            false,
            Epoch::default(),
        );

        // Create rent account
        let rent_key = rent::id();
        #[derive(BorshSerialize, BorshDeserialize)]
        struct MockRent {
            lamports_per_byte_year: u64,
            exemption_threshold: f64,
            burn_percent: u8,
        }
        let rent_data = &mut MockRent {
            lamports_per_byte_year: Rent::default().lamports_per_byte_year,
            exemption_threshold: Rent::default().exemption_threshold,
            burn_percent: Rent::default().burn_percent,
        }
        .try_to_vec()
        .unwrap();
        let rent_bal = &mut Rent::default().minimum_balance(mem::size_of::<MockRent>());
        let rent = AccountInfo::new(
            &rent_key,
            false,
            false,
            rent_bal,
            rent_data,
            &no_account,
            false,
            Epoch::default(),
        );

        // Create token mint account
        let mint_key = &Pubkey::new_unique();
        let mint_bal = &mut Rent::default().minimum_balance(mem::size_of::<Mint>());
        let mint_data = &mut vec![0; Mint::LEN];
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
        let wallet_key = &Pubkey::new_unique();
        let wallet_bal = &mut Rent::default().minimum_balance(mem::size_of::<Account>());
        let wallet_data = &mut vec![0; Account::LEN];
        let wallet = AccountInfo::new(
            wallet_key,
            false,
            true,
            wallet_bal,
            wallet_data,
            spl_id,
            false,
            Epoch::default(),
        );
        spl_token::processor::Processor::process(
            spl_id,
            &[wallet.clone(), mint.clone(), rent.clone()],
            &TokenInstruction::InitializeAccount2 { owner: vester_key }.pack(),
        )
        .unwrap();
        {
            let mut data = Account::unpack_from_slice(&wallet.try_borrow_data().unwrap()).unwrap();
            data.amount = 1000000000000;
            data.pack_into_slice(*wallet.try_borrow_mut_data().unwrap());
        }

        // Create token wallet account
        let receiver_key = &Pubkey::new_unique();
        let receiver_bal = &mut Rent::default().minimum_balance(mem::size_of::<Account>());
        let receiver_data = &mut vec![0; Account::LEN];
        let receiver = AccountInfo::new(
            receiver_key,
            false,
            true,
            receiver_bal,
            receiver_data,
            spl_id,
            false,
            Epoch::default(),
        );
        spl_token::processor::Processor::process(
            spl_id,
            &[receiver.clone(), mint.clone(), rent.clone()],
            &TokenInstruction::InitializeAccount2 { owner: claimer_key }.pack(),
        )
        .unwrap();

        let nonce = 1u64;

        // Create vesting pda account
        let (vesting_key, _) = Pubkey::find_program_address(
            &[
                "VESTING".as_bytes(),
                &mint.key.to_bytes(),
                &claimer.key.to_bytes(),
                &nonce.to_le_bytes(),
            ],
            &program_id,
        );
        let vesting_bal = &mut Rent::default().minimum_balance(mem::size_of::<Vesting>());
        let vesting_data = &mut vec![0; mem::size_of::<Vesting>()];
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
        let (vault_key, _) = Pubkey::find_program_address(
            &[
                "VAULT".as_bytes(),
                &mint.key.to_bytes(),
                &claimer.key.to_bytes(),
                &nonce.to_le_bytes(),
            ],
            &program_id,
        );
        let vault_bal = &mut Rent::default().minimum_balance(mem::size_of::<Account>());
        let vault_data = &mut vec![0; mem::size_of::<Account>()];
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

        // Create Vesting
        let binding = [
            vester.clone(),
            mint.clone(),
            wallet.clone(),
            vesting.clone(),
            vault.clone(),
        ];
        process(
            program_id,
            &binding,
            &VestingInstruction::CreateVesting {
                user: claimer_key,
                nonce: 1,
                amount: 15000,
                start: (Clock {
                    unix_timestamp: 60 * 60 * 24 * 365,
                    ..Clock::default()
                }
                .unix_timestamp
                    - 100) as u64,
                cliff: 0,
                duration: 150,
            }
            .try_to_vec()
            .unwrap(),
        )
        .unwrap();

        // Claim Vesting
        let binding = [
            claimer.clone(),
            mint.clone(),
            receiver.clone(),
            vesting.clone(),
            vault.clone(),
        ];
        process(
            program_id,
            &binding,
            &VestingInstruction::Claim {
                user: claimer_key,
                nonce: 1,
            }
            .try_to_vec()
            .unwrap(),
        )
        .unwrap();

        // Check Vesting account state change
        assert!(Vesting::get_data(&vesting).unwrap().claimed == 10000);

        // In the test `sol_invoke_signed()`` is not available, so we can't check result balances
    }
}
