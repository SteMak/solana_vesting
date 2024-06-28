// use borsh::BorshSerialize;
// use solana_program_test::*;
// use solana_sdk::{
//     account::Account,
//     clock::Clock,
//     instruction::{AccountMeta, Instruction},
//     program_pack::Pack,
//     pubkey::Pubkey,
//     signature::{Keypair, Signer},
//     system_instruction,
//     sysvar::{clock, rent},
//     transaction::Transaction,
// };
// use solana_vesting::{
//     instruction::VestingInstruction,
//     pda::{Vault, Vesting},
//     process_instruction,
// };
// use spl_token::state::Mint;
// use std::mem;

// #[tokio::test]
// async fn test_solana_vesting() {
//     // This doesn't work for some reason

//     let program_id = Pubkey::new_unique();
//     let vester = Keypair::new();
//     let claimer = Keypair::new();
//     let nonce = 3u64;

//     let mut program_test = ProgramTest::new(
//         "solana_vesting",
//         program_id,
//         processor!(process_instruction),
//     );
//     program_test.add_program(
//         "spl_token",
//         spl_token::id(),
//         processor!(spl_token::processor::Processor::process),
//     );

//     let mint_key = Pubkey::new_unique();
//     program_test.add_account(
//         mint_key,
//         Account {
//             lamports: 50000,
//             data: vec![0; mem::size_of::<Mint>()],
//             owner: spl_token::id(),
//             ..Account::default()
//         },
//     );

//     let wallet_key = Pubkey::new_unique();
//     let wallet_data = &mut [0; mem::size_of::<spl_token::state::Account>()];
//     spl_token::state::Account {
//         mint: mint_key,
//         owner: vester.pubkey(),
//         amount: 100000000,
//         state: spl_token::state::AccountState::Initialized,
//         ..Default::default()
//     }
//     .pack_into_slice(wallet_data);
//     program_test.add_account(
//         wallet_key,
//         Account {
//             lamports: 50000,
//             data: wallet_data.into(),
//             owner: spl_token::id(),
//             ..Account::default()
//         },
//     );

//     let receiver_key = Pubkey::new_unique();
//     let receiver_data = &mut [0; mem::size_of::<spl_token::state::Account>()];
//     spl_token::state::Account {
//         mint: mint_key,
//         owner: claimer.pubkey(),
//         amount: 0,
//         state: spl_token::state::AccountState::Initialized,
//         ..Default::default()
//     }
//     .pack_into_slice(receiver_data);

//     program_test.add_account(
//         receiver_key,
//         Account {
//             lamports: 50000,
//             data: receiver_data.into(),
//             owner: spl_token::id(),
//             ..Account::default()
//         },
//     );

//     let (vesting_key, _) = Pubkey::find_program_address(
//         &[
//             "VESTING".as_bytes(),
//             &mint_key.to_bytes(),
//             &claimer.pubkey().to_bytes(),
//             &nonce.to_le_bytes(),
//         ],
//         &program_id,
//     );
//     program_test.add_account(
//         vesting_key,
//         Account {
//             lamports: 50000,
//             data: vec![0; mem::size_of::<Vesting>()],
//             owner: program_id,
//             ..Account::default()
//         },
//     );

//     let (vault_key, _) = Pubkey::find_program_address(
//         &[
//             "VAULT".as_bytes(),
//             &mint_key.to_bytes(),
//             &claimer.pubkey().to_bytes(),
//             &nonce.to_le_bytes(),
//         ],
//         &program_id,
//     );
//     program_test.add_account(
//         vault_key,
//         Account {
//             lamports: 50000,
//             data: vec![0; mem::size_of::<Vault>()],
//             owner: program_id,
//             ..Account::default()
//         },
//     );

//     let mut clock = Clock {
//         unix_timestamp: 50 * 365 * 24 * 60 * 60,
//         ..Clock::default()
//     };
//     program_test.add_sysvar_account(vault_key, &clock);

//     let mut context = program_test.start_with_context().await;

//     let recent_blockhash = context
//         .banks_client
//         .get_new_latest_blockhash(&context.last_blockhash)
//         .await
//         .unwrap();
//     context
//         .banks_client
//         .process_transaction(Transaction::new_signed_with_payer(
//             &[system_instruction::transfer(
//                 &context.payer.pubkey(),
//                 &vester.pubkey(),
//                 1000000000,
//             )],
//             Some(&context.payer.pubkey()),
//             &[&context.payer],
//             recent_blockhash,
//         ))
//         .await
//         .unwrap();

//     let recent_blockhash = context
//         .banks_client
//         .get_new_latest_blockhash(&context.last_blockhash)
//         .await
//         .unwrap();
//     context
//         .banks_client
//         .process_transaction(Transaction::new_signed_with_payer(
//             &[system_instruction::transfer(
//                 &context.payer.pubkey(),
//                 &claimer.pubkey(),
//                 1000000000,
//             )],
//             Some(&context.payer.pubkey()),
//             &[&context.payer],
//             recent_blockhash,
//         ))
//         .await
//         .unwrap();

//     let recent_blockhash = context
//         .banks_client
//         .get_new_latest_blockhash(&context.last_blockhash)
//         .await
//         .unwrap();
//     context
//         .banks_client
//         .process_transaction(Transaction::new_signed_with_payer(
//             &[Instruction::new_with_bytes(
//                 program_id,
//                 &VestingInstruction::CreateVesting {
//                     user: claimer.pubkey(),
//                     nonce,
//                     amount: 1000000,
//                     start: clock.unix_timestamp as u64,
//                     cliff: 100,
//                     duration: 200,
//                 }
//                 .try_to_vec()
//                 .unwrap(),
//                 vec![
//                     AccountMeta::new_readonly(rent::id(), false),
//                     AccountMeta::new(vester.pubkey(), true),
//                     AccountMeta::new_readonly(mint_key, false),
//                     AccountMeta::new(wallet_key, false),
//                     AccountMeta::new(vesting_key, false),
//                     AccountMeta::new(vault_key, false),
//                 ],
//             )],
//             Some(&vester.pubkey()),
//             &[&vester],
//             recent_blockhash,
//         ))
//         .await
//         .unwrap();

//     clock.unix_timestamp += 150;
//     context.set_sysvar(&clock);

//     let recent_blockhash = context
//         .banks_client
//         .get_new_latest_blockhash(&context.last_blockhash)
//         .await
//         .unwrap();
//     context
//         .banks_client
//         .process_transaction(Transaction::new_signed_with_payer(
//             &[Instruction::new_with_bytes(
//                 program_id,
//                 &VestingInstruction::Claim {
//                     user: claimer.pubkey(),
//                     nonce,
//                 }
//                 .try_to_vec()
//                 .unwrap(),
//                 vec![
//                     AccountMeta::new_readonly(clock::id(), false),
//                     AccountMeta::new(claimer.pubkey(), false),
//                     AccountMeta::new_readonly(mint_key, false),
//                     AccountMeta::new(receiver_key, false),
//                     AccountMeta::new(vesting_key, false),
//                     AccountMeta::new(vault_key, false),
//                 ],
//             )],
//             Some(&claimer.pubkey()),
//             &[&claimer],
//             recent_blockhash,
//         ))
//         .await
//         .unwrap();
// }
