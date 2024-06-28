use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, sysvar::rent::Rent,
};
use spl_token::state::Account;

use crate::helpers::*;

/// Vault PDA type
pub struct Vault;

/// Vault PDA seed prefix
const VAULT_SEED: &[u8] = "VAULT".as_bytes();

impl PDA for Vault {
    /// Get account data size
    fn size() -> usize {
        std::mem::size_of::<Account>()
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
        rent: &Rent,
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
            rent,
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
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone)]
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
        std::mem::size_of::<Vesting>()
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
        rent: &Rent,
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
            rent,
            payer,
            program_id,
        )
    }

    /// Get Vesting PDA data
    pub fn get_data(info: &AccountInfo) -> Result<Vesting, ProgramError> {
        let data = Vesting::try_from_slice(&info.data.borrow())
            .map_err(|x| ProgramError::BorshIoError(x.to_string()))?;

        Ok(data)
    }

    /// Set Vesting PDA data
    pub fn set_data(self, info: &AccountInfo) -> Result<(), ProgramError> {
        self.serialize(&mut &mut info.data.borrow_mut()[..])
            .map_err(|x| ProgramError::BorshIoError(x.to_string()))?;

        Ok(())
    }
}

/// Sanity tests
#[cfg(test)]
mod test {
    use std::mem;

    use crate::pda::Vesting;

    use solana_sdk::{account_info::AccountInfo, clock::Epoch, pubkey::Pubkey};

    use super::Vault;

    #[test]
    fn test_check_info() {
        let program_id = &Pubkey::new_unique();
        let nonce = 10u64;

        let mint = Pubkey::new_unique();
        let user = Pubkey::new_unique();

        let (vesting_key, _) = Pubkey::find_program_address(
            &[
                "VESTING".as_bytes(),
                &mint.to_bytes(),
                &user.to_bytes(),
                &nonce.to_le_bytes(),
            ],
            program_id,
        );
        let vesting_bal = &mut 100;
        let vesting_data = &mut [0; 100];
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

        let (vault_key, _) = Pubkey::find_program_address(
            &[
                "VAULT".as_bytes(),
                &mint.to_bytes(),
                &user.to_bytes(),
                &nonce.to_le_bytes(),
            ],
            program_id,
        );
        let vault_bal = &mut 100;
        let vault_data = &mut [0; 100];
        let vault = AccountInfo::new(
            &vault_key,
            false,
            true,
            vault_bal,
            vault_data,
            program_id,
            false,
            Epoch::default(),
        );

        Vesting::check_info(&vesting, program_id, &mint, user, nonce).unwrap();
        Vault::check_info(&vesting, program_id, &mint, user, nonce).unwrap_err();
        Vesting::check_info(&vault, program_id, &mint, user, nonce).unwrap_err();
        Vault::check_info(&vault, program_id, &mint, user, nonce).unwrap();
    }

    #[test]
    fn test_set_get() {
        let program_id = &Pubkey::new_unique();

        let vesting_key = Pubkey::new_unique();
        let vesting_bal = &mut 100;
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

        let data = Vesting {
            amount: 100,
            claimed: 10,
            cliff: 20,
            duration: 50,
            start: 0,
        };
        data.clone().set_data(&vesting).unwrap();
        assert!(Vesting::get_data(&vesting).unwrap() == data);

        let bad_vesting_key = Pubkey::new_unique();
        let bad_vesting_bal = &mut 100;
        let bad_vesting_data = &mut [0; mem::size_of::<Vesting>() - 10];
        let bad_vesting = AccountInfo::new(
            &bad_vesting_key,
            false,
            true,
            bad_vesting_bal,
            bad_vesting_data,
            program_id,
            false,
            Epoch::default(),
        );

        data.clone().set_data(&bad_vesting).unwrap_err();
        Vesting::get_data(&bad_vesting).unwrap_err();
    }
}
