use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    pubkey::Pubkey,
    sysvar::{clock::Clock, rent::Rent},
};

use crate::pda::{Distribute, Vault, Vesting, PDA};

/// Instruction enum definition
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum VestingInstruction {
    CreateVesting {
        beneficiary: Pubkey,
        amount: u64,

        start: u64,
        cliff: u64,
        duration: u64,
    },

    Claim {
        seed_key: Pubkey,
    },
}

/// Structured CreateVesting instruction account infos
pub struct CreateVestingAccounts<'a, 'b, 'c> {
    // [sysvar]
    pub rent: &'c Rent,

    // [signer writeble]
    pub signer: &'a AccountInfo<'b>,
    // [signer]
    pub seed: &'a AccountInfo<'b>,

    // [token_mint]
    pub mint: &'a AccountInfo<'b>,

    // [pda writeble]
    pub vesting: &'c mut PDA<'a, 'b, Vesting>,
    // [pda writeble token_wallet]
    pub vault: &'c mut PDA<'a, 'b, Vault>,
    // [pda writeble token_wallet]
    pub distribute: &'c mut PDA<'a, 'b, Distribute>,
}

/// Structured Claim instruction account infos
pub struct ClaimAccounts<'a, 'b, 'c> {
    // [sysvar]
    pub clock: &'c Clock,

    // [pda writeble]
    pub vesting: &'c mut PDA<'a, 'b, Vesting>,
    // [pda writeble token_wallet]
    pub vault: &'c mut PDA<'a, 'b, Vault>,
    // [pda writeble token_wallet]
    pub distribute: &'c mut PDA<'a, 'b, Distribute>,
}
