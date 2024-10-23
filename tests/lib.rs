use borsh::BorshSerialize;
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    clock::Clock,
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction, system_program,
    sysvar::{clock, rent},
    transaction::Transaction,
};
use solana_vesting::{instruction::VestingInstruction, process_instruction};
use spl_token::state::Mint;

#[tokio::test]
async fn test_solana_vesting() {
    // This doesn't work for some reason

    let program_id = Pubkey::new_unique();
    let vester = Keypair::new();
    let claimer = Keypair::new();
    let seed = Keypair::new();

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 10000000000000000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    program_test.add_account(
        mint_key,
        Account {
            lamports: 50000,
            data: mint_data.into(),
            owner: spl_token::id(),
            ..Account::default()
        },
    );

    let receiver_key = Pubkey::new_unique();
    let receiver_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: claimer.pubkey(),
        amount: 0,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(receiver_data);

    program_test.add_account(
        receiver_key,
        Account {
            lamports: 50000,
            data: receiver_data.into(),
            owner: spl_token::id(),
            ..Account::default()
        },
    );

    let (vesting_key, _) = Pubkey::find_program_address(
        &["VESTING".as_bytes(), &seed.pubkey().as_ref()],
        &program_id,
    );

    let (vault_key, _) =
        Pubkey::find_program_address(&["VAULT".as_bytes(), &seed.pubkey().as_ref()], &program_id);

    let (distribute_key, _) = Pubkey::find_program_address(
        &["DISTRIBUTE".as_bytes(), seed.pubkey().as_ref()],
        &program_id,
    );

    let mut clock = Clock {
        unix_timestamp: 50 * 365 * 24 * 60 * 60,
        ..Clock::default()
    };
    program_test.add_sysvar_account(clock::id(), &clock);

    let mut context = program_test.start_with_context().await;

    let recent_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    context
        .banks_client
        .process_transaction(Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &context.payer.pubkey(),
                &vester.pubkey(),
                1000000000,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            recent_blockhash,
        ))
        .await
        .unwrap();

    let recent_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    context
        .banks_client
        .process_transaction(Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &context.payer.pubkey(),
                &claimer.pubkey(),
                1000000000,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            recent_blockhash,
        ))
        .await
        .unwrap();

    let recent_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    context
        .banks_client
        .process_transaction(Transaction::new_signed_with_payer(
            &[Instruction::new_with_bytes(
                program_id,
                &VestingInstruction::CreateVesting {
                    beneficiary: claimer.pubkey(),
                    amount: 1000000,
                    start: clock.unix_timestamp as u64,
                    cliff: 100,
                    duration: 200,
                }
                .try_to_vec()
                .unwrap(),
                vec![
                    AccountMeta::new_readonly(rent::id(), false),
                    AccountMeta::new(vester.pubkey(), true),
                    AccountMeta::new(seed.pubkey(), true),
                    AccountMeta::new_readonly(mint_key, false),
                    AccountMeta::new(vesting_key, false),
                    AccountMeta::new(vault_key, false),
                    AccountMeta::new(distribute_key, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                ],
            )],
            Some(&vester.pubkey()),
            &[&vester, &seed],
            recent_blockhash,
        ))
        .await
        .unwrap();

    clock.unix_timestamp += 150;
    context.set_sysvar(&clock);

    let recent_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    context
        .banks_client
        .process_transaction(Transaction::new_signed_with_payer(
            &[Instruction::new_with_bytes(
                program_id,
                &VestingInstruction::Claim {
                    seed_key: seed.pubkey(),
                }
                .try_to_vec()
                .unwrap(),
                vec![
                    AccountMeta::new_readonly(clock::id(), false),
                    AccountMeta::new(vesting_key, false),
                    AccountMeta::new(vault_key, false),
                    AccountMeta::new(distribute_key, false),
                ],
            )],
            Some(&claimer.pubkey()),
            &[&claimer],
            recent_blockhash,
        ))
        .await
        .unwrap();
}
