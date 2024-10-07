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
pub struct CreateVestingAccounts<'a, 'b> {
    // [sysvar]
    pub rent: &'b Rent,

    // [signer writeble]
    pub signer: &'a AccountInfo<'a>,
    // [signer]
    pub seed: &'a AccountInfo<'a>,

    // [token_mint]
    pub mint: &'a AccountInfo<'a>,

    // [pda writeble]
    pub vesting: &'b mut PDA<'a, Vesting>,
    // [pda writeble token_wallet]
    pub vault: &'b mut PDA<'a, Vault>,
    // [pda writeble token_wallet]
    pub distribute: &'b mut PDA<'a, Distribute>,
}

/// Structured Claim instruction account infos
pub struct ClaimAccounts<'a, 'b> {
    // [sysvar]
    pub clock: &'b Clock,

    // [pda writeble]
    pub vesting: &'b mut PDA<'a, Vesting>,
    // [pda writeble token_wallet]
    pub vault: &'b mut PDA<'a, Vault>,
    // [pda writeble token_wallet]
    pub distribute: &'b mut PDA<'a, Distribute>,
}
