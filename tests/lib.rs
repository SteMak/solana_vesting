use borsh::{BorshDeserialize, BorshSerialize};
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
use solana_vesting::{instruction::VestingInstruction, pda::Vesting, process_instruction};
use spl_token::state::Mint;

macro_rules! last_hash {
    ($ctx:expr) => {
        $ctx.get_new_latest_blockhash().await.unwrap()
    };
}

macro_rules! add_account {
    ($pt:expr, $key:expr, $data:expr, $owner:expr) => {
        $pt.add_account(
            $key,
            Account {
                lamports: 1_000_000,
                data: $data.into(),
                owner: $owner,
                ..Account::default()
            },
        );
    };
}

macro_rules! fund_account {
    ($ctx:expr, $account:expr) => {
        let recent_blockhash = last_hash!($ctx);
        $ctx.banks_client
            .process_transaction(Transaction::new_signed_with_payer(
                &[system_instruction::transfer(
                    &$ctx.payer.pubkey(),
                    &$account.pubkey(),
                    10_000_000_000,
                )],
                Some(&$ctx.payer.pubkey()),
                &[&$ctx.payer],
                recent_blockhash,
            ))
            .await
            .unwrap();
    };
}

macro_rules! timeskip {
    ($ctx:expr, $time:expr) => {
        let mut clock = $ctx.banks_client.get_sysvar::<Clock>().await.unwrap();
        clock.unix_timestamp += $time as i64;
        $ctx.set_sysvar(&clock);
    };
}

macro_rules! now {
    ($ctx:expr) => {
        $ctx.banks_client
            .get_sysvar::<Clock>()
            .await
            .unwrap()
            .unix_timestamp as u64
    };
}

macro_rules! execute {
    ($ctx:expr, $program_id:expr, $instruction:expr, $accounts:expr, $payer:expr, $signers:expr) => {{
        timeskip!($ctx, 0);
        let recent_blockhash = last_hash!($ctx);
        $ctx.banks_client
            .process_transaction(Transaction::new_signed_with_payer(
                &[Instruction::new_with_bytes(
                    $program_id,
                    &$instruction,
                    $accounts,
                )],
                Some(&$payer.pubkey()),
                &$signers,
                recent_blockhash,
            ))
            .await
    }};
}

macro_rules! get_accout_data {
    ($ctx:expr, $key:expr) => {{
        timeskip!($ctx, 0);
        &$ctx
            .banks_client
            .get_account($key)
            .await
            .unwrap()
            .unwrap()
            .data
    }};
}

#[tokio::test]
async fn test_no_double_vesting() {
    let program_id = Pubkey::new_unique();

    let vester = Keypair::new();
    let claimer = Keypair::new();

    let seed = Keypair::new();
    let amount = 1_000_000;
    let cliff = 100;
    let duration = 400;

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

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 100_000_000_000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    add_account!(program_test, mint_key, mint_data, spl_token::id());

    let funder_key = Pubkey::new_unique();
    let funder_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: vester.pubkey(),
        amount: 10_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(funder_data);
    add_account!(program_test, funder_key, funder_data, spl_token::id());

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
    add_account!(program_test, receiver_key, receiver_data, spl_token::id());

    let mut context = program_test.start_with_context().await;

    fund_account!(context, vester);
    fund_account!(context, claimer);

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
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
        vester,
        [&vester, &seed]
    )
    .unwrap();

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount: 1,
            start: now,
            cliff,
            duration,
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
        vester,
        [&vester, &seed]
    )
    .unwrap_err();

    let vesting = Vesting::try_from_slice(get_accout_data!(context, vesting_key)).unwrap();
    assert_eq!(vesting.amount, amount);
}

#[tokio::test]
async fn test_no_direct_vault_withdraw() {
    let program_id = Pubkey::new_unique();

    let vester = Keypair::new();
    let claimer = Keypair::new();

    let seed = Keypair::new();
    let amount = 1_000_000;
    let cliff = 100;
    let duration = 400;

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

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 100_000_000_000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    add_account!(program_test, mint_key, mint_data, spl_token::id());

    let funder_key = Pubkey::new_unique();
    let funder_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: vester.pubkey(),
        amount: 10_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(funder_data);
    add_account!(program_test, funder_key, funder_data, spl_token::id());

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
    add_account!(program_test, receiver_key, receiver_data, spl_token::id());

    let mut context = program_test.start_with_context().await;

    fund_account!(context, vester);
    fund_account!(context, claimer);

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
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
        vester,
        [&vester, &seed]
    )
    .unwrap();

    execute!(
        context,
        spl_token::id(),
        spl_token::instruction::TokenInstruction::Transfer { amount }.pack(),
        vec![
            AccountMeta::new(funder_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(spl_token::id(), false),
        ],
        vester,
        [&vester]
    )
    .unwrap();

    execute!(
        context,
        spl_token::id(),
        spl_token::instruction::TokenInstruction::Transfer { amount }.pack(),
        vec![
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(claimer.pubkey(), true),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap_err();

    execute!(
        context,
        spl_token::id(),
        spl_token::instruction::TokenInstruction::Transfer { amount }.pack(),
        vec![
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(spl_token::id(), false),
        ],
        vester,
        [&vester]
    )
    .unwrap_err();
}

#[tokio::test]
async fn test_no_early_claim() {
    let program_id = Pubkey::new_unique();

    let vester = Keypair::new();
    let claimer = Keypair::new();

    let seed = Keypair::new();
    let amount = 1_000_000;
    let cliff = 100;
    let duration = 400;

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

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 100_000_000_000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    add_account!(program_test, mint_key, mint_data, spl_token::id());

    let funder_key = Pubkey::new_unique();
    let funder_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: vester.pubkey(),
        amount: 10_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(funder_data);
    add_account!(program_test, funder_key, funder_data, spl_token::id());

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
    add_account!(program_test, receiver_key, receiver_data, spl_token::id());

    let mut context = program_test.start_with_context().await;

    fund_account!(context, vester);
    fund_account!(context, claimer);

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now + cliff,
            cliff,
            duration,
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
        vester,
        [&vester, &seed]
    )
    .unwrap();

    execute!(
        context,
        spl_token::id(),
        spl_token::instruction::TokenInstruction::Transfer { amount }.pack(),
        vec![
            AccountMeta::new(funder_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(spl_token::id(), false),
        ],
        vester,
        [&vester]
    )
    .unwrap();

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    timeskip!(context, cliff / 2);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    timeskip!(context, cliff);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    timeskip!(context, cliff / 2 - 1);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    let vesting = Vesting::try_from_slice(get_accout_data!(context, vesting_key)).unwrap();
    assert_eq!(vesting.claimed, 0);

    let distribute =
        spl_token::state::Account::unpack(get_accout_data!(context, distribute_key)).unwrap();
    assert_eq!(distribute.amount, 0);

    let vault = spl_token::state::Account::unpack(get_accout_data!(context, vault_key)).unwrap();
    assert_eq!(vault.amount, amount);

    timeskip!(context, 1);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    let vesting = Vesting::try_from_slice(get_accout_data!(context, vesting_key)).unwrap();
    assert_eq!(vesting.claimed, amount * cliff / duration);

    let distribute =
        spl_token::state::Account::unpack(get_accout_data!(context, distribute_key)).unwrap();
    assert_eq!(distribute.amount, amount * cliff / duration);

    let vault = spl_token::state::Account::unpack(get_accout_data!(context, vault_key)).unwrap();
    assert_eq!(vault.amount, amount - amount * cliff / duration);
}

#[tokio::test]
async fn test_no_unauthorized_distribute_withdraw() {
    let program_id = Pubkey::new_unique();

    let vester = Keypair::new();
    let claimer = Keypair::new();

    let seed = Keypair::new();
    let amount = 1_000_000;
    let cliff = 100;
    let duration = 400;

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

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 100_000_000_000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    add_account!(program_test, mint_key, mint_data, spl_token::id());

    let funder_key = Pubkey::new_unique();
    let funder_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: vester.pubkey(),
        amount: 10_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(funder_data);
    add_account!(program_test, funder_key, funder_data, spl_token::id());

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
    add_account!(program_test, receiver_key, receiver_data, spl_token::id());

    let mut context = program_test.start_with_context().await;

    fund_account!(context, vester);
    fund_account!(context, claimer);

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
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
        vester,
        [&vester, &seed]
    )
    .unwrap();

    execute!(
        context,
        spl_token::id(),
        spl_token::instruction::TokenInstruction::Transfer { amount }.pack(),
        vec![
            AccountMeta::new(funder_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(spl_token::id(), false),
        ],
        vester,
        [&vester]
    )
    .unwrap();

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    timeskip!(context, cliff);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    execute!(
        context,
        spl_token::id(),
        spl_token::instruction::TokenInstruction::Transfer {
            amount: amount * cliff / duration
        }
        .pack(),
        vec![
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(receiver_key, false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(spl_token::id(), false),
        ],
        vester,
        [&vester]
    )
    .unwrap_err();
}

#[tokio::test]
async fn test_distribute_withdraw() {
    let program_id = Pubkey::new_unique();

    let vester = Keypair::new();
    let claimer = Keypair::new();

    let seed = Keypair::new();
    let amount = 1_000_000;
    let cliff = 100;
    let duration = 400;

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

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 100_000_000_000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    add_account!(program_test, mint_key, mint_data, spl_token::id());

    let funder_key = Pubkey::new_unique();
    let funder_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: vester.pubkey(),
        amount: 10_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(funder_data);
    add_account!(program_test, funder_key, funder_data, spl_token::id());

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
    add_account!(program_test, receiver_key, receiver_data, spl_token::id());

    let mut context = program_test.start_with_context().await;

    fund_account!(context, vester);
    fund_account!(context, claimer);

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
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
        vester,
        [&vester, &seed]
    )
    .unwrap();

    execute!(
        context,
        spl_token::id(),
        spl_token::instruction::TokenInstruction::Transfer { amount }.pack(),
        vec![
            AccountMeta::new(funder_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(spl_token::id(), false),
        ],
        vester,
        [&vester]
    )
    .unwrap();

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    timeskip!(context, cliff);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    execute!(
        context,
        spl_token::id(),
        spl_token::instruction::TokenInstruction::Transfer {
            amount: amount * cliff / duration
        }
        .pack(),
        vec![
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(receiver_key, false),
            AccountMeta::new(claimer.pubkey(), true),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    let vesting = Vesting::try_from_slice(get_accout_data!(context, vesting_key)).unwrap();
    assert_eq!(vesting.claimed, amount * cliff / duration);

    let receiver =
        spl_token::state::Account::unpack(get_accout_data!(context, receiver_key)).unwrap();
    assert_eq!(receiver.amount, amount * cliff / duration);

    let distribute =
        spl_token::state::Account::unpack(get_accout_data!(context, distribute_key)).unwrap();
    assert_eq!(distribute.amount, 0);

    let vault = spl_token::state::Account::unpack(get_accout_data!(context, vault_key)).unwrap();
    assert_eq!(vault.amount, amount - amount * cliff / duration);

    timeskip!(context, duration);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    execute!(
        context,
        spl_token::id(),
        spl_token::instruction::TokenInstruction::Transfer {
            amount: amount - amount * cliff / duration
        }
        .pack(),
        vec![
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(receiver_key, false),
            AccountMeta::new(claimer.pubkey(), true),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    let vesting = Vesting::try_from_slice(get_accout_data!(context, vesting_key)).unwrap();
    assert_eq!(vesting.claimed, amount);

    let receiver =
        spl_token::state::Account::unpack(get_accout_data!(context, receiver_key)).unwrap();
    assert_eq!(receiver.amount, amount);

    let distribute =
        spl_token::state::Account::unpack(get_accout_data!(context, distribute_key)).unwrap();
    assert_eq!(distribute.amount, 0);

    let vault = spl_token::state::Account::unpack(get_accout_data!(context, vault_key)).unwrap();
    assert_eq!(vault.amount, 0);
}

#[tokio::test]
async fn test_missing_signer() {
    let program_id = Pubkey::new_unique();

    let vester = Keypair::new();
    let claimer = Keypair::new();

    let seed = Keypair::new();
    let amount = 1_000_000;
    let cliff = 100;
    let duration = 400;

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

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 100_000_000_000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    add_account!(program_test, mint_key, mint_data, spl_token::id());

    let funder_key = Pubkey::new_unique();
    let funder_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: vester.pubkey(),
        amount: 10_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(funder_data);
    add_account!(program_test, funder_key, funder_data, spl_token::id());

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
    add_account!(program_test, receiver_key, receiver_data, spl_token::id());

    let mut context = program_test.start_with_context().await;

    fund_account!(context, vester);
    fund_account!(context, claimer);

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new(vester.pubkey(), false),
            AccountMeta::new(seed.pubkey(), true),
            AccountMeta::new_readonly(mint_key, false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        claimer,
        [&claimer, &seed]
    )
    .unwrap_err();

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(seed.pubkey(), false),
            AccountMeta::new_readonly(mint_key, false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        vester,
        [&vester]
    )
    .unwrap_err();
}

#[tokio::test]
async fn test_wrong_pda() {
    let program_id = Pubkey::new_unique();

    let vester = Keypair::new();
    let claimer = Keypair::new();

    let seed = Keypair::new();
    let amount = 1_000_000;
    let cliff = 100;
    let duration = 400;

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

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 100_000_000_000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    add_account!(program_test, mint_key, mint_data, spl_token::id());

    let funder_key = Pubkey::new_unique();
    let funder_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: vester.pubkey(),
        amount: 10_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(funder_data);
    add_account!(program_test, funder_key, funder_data, spl_token::id());

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
    add_account!(program_test, receiver_key, receiver_data, spl_token::id());

    let mut context = program_test.start_with_context().await;

    fund_account!(context, vester);
    fund_account!(context, claimer);

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(seed.pubkey(), true),
            AccountMeta::new_readonly(mint_key, false),
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        vester,
        [&vester, &seed]
    )
    .unwrap_err();

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(seed.pubkey(), true),
            AccountMeta::new_readonly(mint_key, false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        vester,
        [&vester, &seed]
    )
    .unwrap_err();

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
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
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        vester,
        [&vester, &seed]
    )
    .unwrap_err();

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
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
        vester,
        [&vester, &seed]
    )
    .unwrap();

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap_err();

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap_err();

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap_err();
}

#[tokio::test]
async fn test_wrong_instruction() {
    let program_id = Pubkey::new_unique();

    let vester = Keypair::new();
    let claimer = Keypair::new();

    let seed = Keypair::new();
    let amount = 1_000_000;
    let cliff = 100;
    let duration = 400;

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

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 100_000_000_000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    add_account!(program_test, mint_key, mint_data, spl_token::id());

    let funder_key = Pubkey::new_unique();
    let funder_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: vester.pubkey(),
        amount: 10_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(funder_data);
    add_account!(program_test, funder_key, funder_data, spl_token::id());

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
    add_account!(program_test, receiver_key, receiver_data, spl_token::id());

    let mut context = program_test.start_with_context().await;

    fund_account!(context, vester);
    fund_account!(context, claimer);

    execute!(
        context,
        program_id,
        Vesting {
            ..Default::default()
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new(vester.pubkey(), false),
            AccountMeta::new(seed.pubkey(), true),
            AccountMeta::new_readonly(mint_key, false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        vester,
        [&vester, &seed]
    )
    .unwrap_err();

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new(vester.pubkey(), false),
            AccountMeta::new(seed.pubkey(), true),
            AccountMeta::new_readonly(mint_key, false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        vester,
        [&vester, &seed]
    )
    .unwrap();
}

#[tokio::test]
async fn test_missing_accounts() {
    let program_id = Pubkey::new_unique();

    let vester = Keypair::new();
    let claimer = Keypair::new();

    let seed = Keypair::new();
    let amount = 1_000_000;
    let cliff = 100;
    let duration = 400;

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

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 100_000_000_000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    add_account!(program_test, mint_key, mint_data, spl_token::id());

    let funder_key = Pubkey::new_unique();
    let funder_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: vester.pubkey(),
        amount: 10_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(funder_data);
    add_account!(program_test, funder_key, funder_data, spl_token::id());

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
    add_account!(program_test, receiver_key, receiver_data, spl_token::id());

    let mut context = program_test.start_with_context().await;

    fund_account!(context, vester);
    fund_account!(context, claimer);

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
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
        ],
        vester,
        [&vester, &seed]
    )
    .unwrap_err();

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(seed.pubkey(), true),
        ],
        vester,
        [&vester, &seed]
    )
    .unwrap_err();
}

#[tokio::test]
async fn test_low_funded() {
    let program_id = Pubkey::new_unique();

    let vester = Keypair::new();
    let claimer = Keypair::new();

    let seed = Keypair::new();
    let amount = 1_000_000;
    let cliff = 100;
    let duration = 400;

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

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 100_000_000_000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    add_account!(program_test, mint_key, mint_data, spl_token::id());

    let funder_key = Pubkey::new_unique();
    let funder_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: vester.pubkey(),
        amount: 10_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(funder_data);
    add_account!(program_test, funder_key, funder_data, spl_token::id());

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
    add_account!(program_test, receiver_key, receiver_data, spl_token::id());

    let mut context = program_test.start_with_context().await;

    fund_account!(context, vester);
    fund_account!(context, claimer);

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
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
        vester,
        [&vester, &seed]
    )
    .unwrap();

    execute!(
        context,
        spl_token::id(),
        spl_token::instruction::TokenInstruction::Transfer { amount: amount / 3 }.pack(),
        vec![
            AccountMeta::new(funder_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(spl_token::id(), false),
        ],
        vester,
        [&vester]
    )
    .unwrap();

    timeskip!(context, cliff);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    let vesting = Vesting::try_from_slice(get_accout_data!(context, vesting_key)).unwrap();
    assert_eq!(vesting.claimed, amount * cliff / duration);

    let distribute =
        spl_token::state::Account::unpack(get_accout_data!(context, distribute_key)).unwrap();
    assert_eq!(distribute.amount, amount * cliff / duration);

    let vault = spl_token::state::Account::unpack(get_accout_data!(context, vault_key)).unwrap();
    assert_eq!(vault.amount, amount / 3 - amount * cliff / duration);

    timeskip!(context, duration);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    let vesting = Vesting::try_from_slice(get_accout_data!(context, vesting_key)).unwrap();
    assert_eq!(vesting.claimed, amount / 3);

    let distribute =
        spl_token::state::Account::unpack(get_accout_data!(context, distribute_key)).unwrap();
    assert_eq!(distribute.amount, amount / 3);

    let vault = spl_token::state::Account::unpack(get_accout_data!(context, vault_key)).unwrap();
    assert_eq!(vault.amount, 0);
}

#[tokio::test]
async fn test_over_funded() {
    let program_id = Pubkey::new_unique();

    let vester = Keypair::new();
    let claimer = Keypair::new();

    let seed = Keypair::new();
    let amount = 1_000_000;
    let cliff = 100;
    let duration = 400;

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

    let mut program_test = ProgramTest::new(
        "solana_vesting",
        program_id,
        processor!(process_instruction),
    );

    let mint_key = Pubkey::new_unique();
    let mint_data = &mut [0; Mint::LEN];
    spl_token::state::Mint {
        is_initialized: true,
        supply: 100_000_000_000,
        ..Default::default()
    }
    .pack_into_slice(mint_data);
    add_account!(program_test, mint_key, mint_data, spl_token::id());

    let funder_key = Pubkey::new_unique();
    let funder_data = &mut [0; spl_token::state::Account::LEN];
    spl_token::state::Account {
        mint: mint_key,
        owner: vester.pubkey(),
        amount: 10_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    }
    .pack_into_slice(funder_data);
    add_account!(program_test, funder_key, funder_data, spl_token::id());

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
    add_account!(program_test, receiver_key, receiver_data, spl_token::id());

    let mut context = program_test.start_with_context().await;

    fund_account!(context, vester);
    fund_account!(context, claimer);

    let now = now!(context);
    execute!(
        context,
        program_id,
        VestingInstruction::CreateVesting {
            beneficiary: claimer.pubkey(),
            amount,
            start: now,
            cliff,
            duration,
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
        vester,
        [&vester, &seed]
    )
    .unwrap();

    execute!(
        context,
        spl_token::id(),
        spl_token::instruction::TokenInstruction::Transfer { amount: amount * 3 }.pack(),
        vec![
            AccountMeta::new(funder_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(vester.pubkey(), true),
            AccountMeta::new(spl_token::id(), false),
        ],
        vester,
        [&vester]
    )
    .unwrap();

    timeskip!(context, cliff);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    let vesting = Vesting::try_from_slice(get_accout_data!(context, vesting_key)).unwrap();
    assert_eq!(vesting.claimed, amount * cliff / duration);

    let distribute =
        spl_token::state::Account::unpack(get_accout_data!(context, distribute_key)).unwrap();
    assert_eq!(distribute.amount, amount * cliff / duration);

    let vault = spl_token::state::Account::unpack(get_accout_data!(context, vault_key)).unwrap();
    assert_eq!(vault.amount, amount * 3 - amount * cliff / duration);

    timeskip!(context, duration - cliff - 1);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    let vesting = Vesting::try_from_slice(get_accout_data!(context, vesting_key)).unwrap();
    assert_eq!(vesting.claimed, amount * (duration - 1) / duration);

    let distribute =
        spl_token::state::Account::unpack(get_accout_data!(context, distribute_key)).unwrap();
    assert_eq!(distribute.amount, amount * (duration - 1) / duration);

    let vault = spl_token::state::Account::unpack(get_accout_data!(context, vault_key)).unwrap();
    assert_eq!(
        vault.amount,
        amount * 3 - amount * (duration - 1) / duration
    );

    timeskip!(context, 1);

    execute!(
        context,
        program_id,
        VestingInstruction::Claim {
            seed_key: seed.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
        vec![
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new(vesting_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new(distribute_key, false),
            AccountMeta::new(spl_token::id(), false),
        ],
        claimer,
        [&claimer]
    )
    .unwrap();

    let vesting = Vesting::try_from_slice(get_accout_data!(context, vesting_key)).unwrap();
    assert_eq!(vesting.claimed, amount * 3);

    let distribute =
        spl_token::state::Account::unpack(get_accout_data!(context, distribute_key)).unwrap();
    assert_eq!(distribute.amount, amount * 3);

    let vault = spl_token::state::Account::unpack(get_accout_data!(context, vault_key)).unwrap();
    assert_eq!(vault.amount, 0);
}
