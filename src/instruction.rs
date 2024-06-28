use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    pubkey::Pubkey,
    sysvar::{clock::Clock, rent::Rent},
};

/// Instruction enum definition
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

/// Structured CreateVesting instruction account infos
pub struct CreateVestingAccounts<'a, 'b> {
    // [sysvar]
    pub rent: &'a Rent,

    // [signer writeble]
    pub signer: &'a AccountInfo<'b>,
    // [token_mint]
    pub mint: &'a AccountInfo<'b>,
    // [writeble token_wallet]
    pub wallet: &'a AccountInfo<'b>,

    // [pda writeble]
    pub vesting: &'a AccountInfo<'b>,
    // [pda writeble token_wallet]
    pub vault: &'a AccountInfo<'b>,
}

/// Structured Claim instruction account infos
pub struct ClaimAccounts<'a, 'b> {
    // [sysvar]
    pub clock: &'a Clock,

    #[allow(dead_code)]
    // [signer]
    pub signer: &'a AccountInfo<'b>,
    // [token_mint]
    pub mint: &'a AccountInfo<'b>,
    // [writeble token_wallet]
    pub wallet: &'a AccountInfo<'b>,

    // [pda writeble]
    pub vesting: &'a AccountInfo<'b>,
    // [pda writeble token_wallet]
    pub vault: &'a AccountInfo<'b>,
}
