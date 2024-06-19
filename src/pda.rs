use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use spl_token::state::Account;

use crate::helpers::*;

/// Vault PDA type
pub struct Vault;

/// Vault PDA seed prefix
const VAULT_SEED: &[u8] = "VAULT".as_bytes();

impl PDA for Vault {
    /// Get account data size
    fn size() -> usize {
        return std::mem::size_of::<Account>();
    }
}

impl Vault {
    /// Check if provided PDA corresponds Vault seeds
    pub fn check_info(
        pda: &AccountInfo,
        program_id: &Pubkey,
        mint: &Pubkey,
        user: Pubkey,
        nonce: u64,
    ) -> Result<(), ProgramError> {
        check_expected_address(
            pda,
            program_id,
            &[
                VAULT_SEED,
                &mint.to_bytes(),
                &user.to_bytes(),
                &nonce.to_le_bytes(),
            ],
        )
    }

    /// Create Vault PDA
    pub fn create<'a>(
        pda: &AccountInfo<'a>,
        program_id: &Pubkey,
        payer: &AccountInfo<'a>,
        mint: &AccountInfo<'a>,
        user: Pubkey,
        nonce: u64,
    ) -> Result<(), ProgramError> {
        create_pda::<Vault>(
            pda,
            program_id,
            &[
                VAULT_SEED,
                &mint.key.to_bytes(),
                &user.to_bytes(),
                &nonce.to_le_bytes(),
            ],
            payer,
            &spl_token::id(),
        )?;
        init_token_pda(pda, mint)?;

        Ok(())
    }

    /// Transfer spl-token to Vault
    pub fn transfer_in<'a>(
        pda: &AccountInfo<'a>,
        wallet: &AccountInfo<'a>,
        signer: &AccountInfo<'a>,
        amount: u64,
    ) -> Result<(), ProgramError> {
        transfer_to_pda(pda, wallet, signer, amount)
    }

    /// Transfer spl-token from Vault
    pub fn transfer_out<'a>(
        pda: &AccountInfo<'a>,
        program_id: &Pubkey,
        wallet: &AccountInfo<'a>,
        mint: &Pubkey,
        user: Pubkey,
        nonce: u64,
        amount: u64,
    ) -> Result<(), ProgramError> {
        transfer_from_pda(
            pda,
            program_id,
            &[
                VAULT_SEED,
                &mint.to_bytes(),
                &user.to_bytes(),
                &nonce.to_le_bytes(),
            ],
            wallet,
            amount,
        )
    }
}

/// Vesting PDA type
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Vesting {
    pub amount: u64,
    pub claimed: u64,

    pub start: u64,
    pub cliff: u64,
    pub duration: u64,
}

/// Vesting PDA seed prefix
const VESTING_SEED: &[u8] = "VESTING".as_bytes();

impl PDA for Vesting {
    /// Get account data size
    fn size() -> usize {
        return std::mem::size_of::<Vesting>();
    }
}

impl Vesting {
    /// Check if provided PDA corresponds Vesting seeds
    pub fn check_info(
        pda: &AccountInfo,
        program_id: &Pubkey,
        mint: &Pubkey,
        user: Pubkey,
        nonce: u64,
    ) -> Result<(), ProgramError> {
        check_expected_address(
            pda,
            program_id,
            &[
                VESTING_SEED,
                &mint.to_bytes(),
                &user.to_bytes(),
                &nonce.to_le_bytes(),
            ],
        )
    }

    /// Create Vesting PDA
    pub fn create<'a>(
        pda: &AccountInfo<'a>,
        program_id: &Pubkey,
        payer: &AccountInfo<'a>,
        mint: &Pubkey,
        user: Pubkey,
        nonce: u64,
    ) -> Result<(), ProgramError> {
        create_pda::<Vesting>(
            pda,
            program_id,
            &[
                VESTING_SEED,
                &mint.to_bytes(),
                &user.to_bytes(),
                &nonce.to_le_bytes(),
            ],
            payer,
            program_id,
        )
    }

    /// Get Vesting PDA data
    pub fn get_data(info: &AccountInfo) -> Result<Vesting, ProgramError> {
        let data = Vesting::try_from_slice(&info.try_borrow_data()?)
            .map_err(|x| ProgramError::BorshIoError(x.to_string()))?;

        Ok(data)
    }

    /// Set Vesting PDA data
    pub fn set_data(self, info: &AccountInfo) -> Result<(), ProgramError> {
        self.serialize(&mut *info.try_borrow_mut_data()?)
            .map_err(|x| ProgramError::BorshIoError(x.to_string()))?;

        Ok(())
    }
}
