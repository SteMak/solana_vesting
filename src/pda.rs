use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
    sysvar::rent::Rent,
};
use spl_token::state::Account;

use crate::{error::CustomError, helpers::*};

pub trait PDAData {}

pub trait PDAMethods<D: PDAData> {
    fn size() -> usize;
    fn check(&self) -> Result<(), ProgramError>;
    fn write(&mut self, data: D) -> Result<(), ProgramError>;
}

pub struct PDA<'a, D: PDAData> {
    pub info: &'a AccountInfo<'a>,
    program_id: &'a Pubkey,
    seeds: Vec<u8>,
    pub data: D,
}

pub struct Vault {
    pub amount: u64,
}

impl PDAData for Vault {}

impl PDAMethods<Vault> for PDA<'_, Vault> {
    fn size() -> usize {
        std::mem::size_of::<Account>()
    }

    fn check(&self) -> Result<(), ProgramError> {
        check_expected_address(self.info.key, self.program_id, &[&self.seeds])
    }

    fn write(&mut self, _: Vault) -> Result<(), ProgramError> {
        Err(ProgramError::Custom(
            CustomError::WriteToPDAForbidden.into(),
        ))
    }
}

impl<'a> PDA<'a, Vault> {
    pub fn new(
        program_id: &'a Pubkey,
        info: &'a AccountInfo<'a>,
        seed_key: &Pubkey,
    ) -> Result<PDA<'a, Vault>, ProgramError> {
        let pda = PDA {
            info,
            program_id,
            seeds: ["VAULT".as_bytes().to_vec(), seed_key.as_ref().to_vec()].concat(),
            data: Vault {
                amount: Account::unpack_from_slice(&info.data.borrow())
                    .unwrap_or_default()
                    .amount,
            },
        };
        pda.check()?;

        Ok(pda)
    }

    pub fn create(
        &self,
        rent: &Rent,
        payer: &AccountInfo<'a>,
        mint: &AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        create_pda::<PDA<Vault>, Vault>(
            self.program_id,
            self.info,
            &[&self.seeds],
            rent,
            payer,
            &spl_token::id(),
        )?;
        init_token_pda(self.info, mint, self.info.key)?;

        Ok(())
    }

    /// Transfer spl-token from Vault
    pub fn transfer_out(&self, wallet: &AccountInfo<'a>, amount: u64) -> Result<(), ProgramError> {
        transfer_from_pda(self.program_id, self.info, &[&self.seeds], wallet, amount)
    }
}

pub struct Distribute {}

impl PDAData for Distribute {}

impl PDAMethods<Distribute> for PDA<'_, Distribute> {
    fn size() -> usize {
        std::mem::size_of::<Account>()
    }

    fn check(&self) -> Result<(), ProgramError> {
        check_expected_address(self.info.key, self.program_id, &[&self.seeds])
    }

    fn write(&mut self, _: Distribute) -> Result<(), ProgramError> {
        Err(ProgramError::Custom(
            CustomError::WriteToPDAForbidden.into(),
        ))
    }
}

impl<'a> PDA<'a, Distribute> {
    pub fn new(
        program_id: &'a Pubkey,
        info: &'a AccountInfo<'a>,
        seed_key: &Pubkey,
    ) -> Result<PDA<'a, Distribute>, ProgramError> {
        let pda = PDA {
            info,
            program_id,
            seeds: ["DISTRIBUTE".as_bytes().to_vec(), seed_key.as_ref().to_vec()].concat(),
            data: Distribute {},
        };
        pda.check()?;

        Ok(pda)
    }

    pub fn create(
        &mut self,
        rent: &Rent,
        payer: &AccountInfo<'a>,
        mint: &AccountInfo<'a>,
        authority: &Pubkey,
    ) -> Result<(), ProgramError> {
        create_pda::<PDA<Distribute>, Distribute>(
            self.program_id,
            self.info,
            &[&self.seeds],
            rent,
            payer,
            &spl_token::id(),
        )?;
        init_token_pda(self.info, mint, authority)?;

        Ok(())
    }
}

/// Vesting PDA type
#[derive(BorshSerialize, BorshDeserialize, Default, Debug, PartialEq, Clone)]
pub struct Vesting {
    pub receiver: Pubkey,
    pub mint: Pubkey,
    pub seed_key: Pubkey,
    pub creator: Pubkey,

    pub amount: u64,
    pub claimed: u64,

    pub start: u64,
    pub cliff: u64,
    pub duration: u64,
}

impl PDAData for Vesting {}

impl PDAMethods<Vesting> for PDA<'_, Vesting> {
    fn size() -> usize {
        std::mem::size_of::<Account>()
    }

    fn check(&self) -> Result<(), ProgramError> {
        check_expected_address(self.info.key, self.program_id, &[&self.seeds])
    }

    fn write(&mut self, data: Vesting) -> Result<(), ProgramError> {
        self.data = data;
        self.data
            .serialize(&mut &mut self.info.data.borrow_mut()[..])
            .map_err(|x| ProgramError::BorshIoError(x.to_string()))
    }
}

impl<'a> PDA<'a, Vesting> {
    pub fn new(
        program_id: &'a Pubkey,
        info: &'a AccountInfo<'a>,
        seed_key: &Pubkey,
    ) -> Result<PDA<'a, Vesting>, ProgramError> {
        let pda = PDA {
            info,
            program_id,
            seeds: ["VESTING".as_bytes().to_vec(), seed_key.as_ref().to_vec()].concat(),
            data: Vesting::try_from_slice(&info.data.borrow()).unwrap_or_default(),
        };
        pda.check()?;

        Ok(pda)
    }

    /// Create Vesting PDA
    pub fn create(&self, rent: &Rent, payer: &AccountInfo<'a>) -> Result<(), ProgramError> {
        create_pda::<PDA<Vesting>, Vesting>(
            self.program_id,
            self.info,
            &[&self.seeds],
            rent,
            payer,
            self.program_id,
        )
    }
}

// /// Sanity tests
// #[cfg(test)]
// mod test {
//     use std::mem;

//     use crate::pda::Vesting;

//     use solana_sdk::{account_info::AccountInfo, clock::Epoch, pubkey::Pubkey};

//     use super::Vault;

//     #[test]
//     fn test_check_info() {
//         let program_id = &Pubkey::new_unique();
//         let nonce = 10u64;

//         let mint = Pubkey::new_unique();
//         let user = Pubkey::new_unique();

//         let (vesting_key, _) = Pubkey::find_program_address(
//             &[
//                 "VESTING".as_bytes(),
//                 &mint.to_bytes(),
//                 &user.to_bytes(),
//                 &nonce.to_le_bytes(),
//             ],
//             program_id,
//         );
//         let vesting_bal = &mut 100;
//         let vesting_data = &mut [0; 100];
//         let vesting = AccountInfo::new(
//             &vesting_key,
//             false,
//             true,
//             vesting_bal,
//             vesting_data,
//             program_id,
//             false,
//             Epoch::default(),
//         );

//         let (vault_key, _) = Pubkey::find_program_address(
//             &[
//                 "VAULT".as_bytes(),
//                 &mint.to_bytes(),
//                 &user.to_bytes(),
//                 &nonce.to_le_bytes(),
//             ],
//             program_id,
//         );
//         let vault_bal = &mut 100;
//         let vault_data = &mut [0; 100];
//         let vault = AccountInfo::new(
//             &vault_key,
//             false,
//             true,
//             vault_bal,
//             vault_data,
//             program_id,
//             false,
//             Epoch::default(),
//         );

//         Vesting::check_info(&vesting, program_id, &mint, user, nonce).unwrap();
//         Vault::check_info(&vesting, program_id, &mint, user, nonce).unwrap_err();
//         Vesting::check_info(&vault, program_id, &mint, user, nonce).unwrap_err();
//         Vault::check_info(&vault, program_id, &mint, user, nonce).unwrap();
//     }

//     #[test]
//     fn test_set_get() {
//         let program_id = &Pubkey::new_unique();

//         let vesting_key = Pubkey::new_unique();
//         let vesting_bal = &mut 100;
//         let vesting_data = &mut [0; mem::size_of::<Vesting>()];
//         let vesting = AccountInfo::new(
//             &vesting_key,
//             false,
//             true,
//             vesting_bal,
//             vesting_data,
//             program_id,
//             false,
//             Epoch::default(),
//         );

//         let data = Vesting {
//             amount: 100,
//             claimed: 10,
//             cliff: 20,
//             duration: 50,
//             start: 0,
//         };
//         data.clone().set_data(&vesting).unwrap();
//         assert!(Vesting::get_data(&vesting).unwrap() == data);

//         let bad_vesting_key = Pubkey::new_unique();
//         let bad_vesting_bal = &mut 100;
//         let bad_vesting_data = &mut [0; mem::size_of::<Vesting>() - 10];
//         let bad_vesting = AccountInfo::new(
//             &bad_vesting_key,
//             false,
//             true,
//             bad_vesting_bal,
//             bad_vesting_data,
//             program_id,
//             false,
//             Epoch::default(),
//         );

//         data.clone().set_data(&bad_vesting).unwrap_err();
//         Vesting::get_data(&bad_vesting).unwrap_err();
//     }
// }
