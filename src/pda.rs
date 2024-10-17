use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
    sysvar::rent::Rent,
};
use spl_token::state::Account;

use crate::{error::CustomError, helpers::*};

/// Generalized PDA structure
pub struct PDA<'a, D: PDAData> {
    pub info: &'a AccountInfo<'a>,
    pub data: D,
    pub program_id: &'a Pubkey,
    // Max 18 seeds each of max 32 bytes, https://docs.rs/solana-program/1.18.17/src/solana_program/pubkey.rs.html#585-592
    pub seeds: Vec<Vec<u8>>,
}

/// Hide the Vec<Vec<us>> -> &[&[u8]] conversion overhead
macro_rules! seeds_convert {
    ($vec:expr) => {
        $vec.iter()
            .map(|v| v.as_slice())
            .collect::<Vec<_>>()
            .as_slice()
    };
}

/// Generalized PDA methods
pub trait PDAMethods<D: PDAData> {
    /// Size of data to be allocated in PDA
    fn size() -> usize;

    /// Validate the pubkey matches the seeds
    fn check(&self) -> Result<(), ProgramError>;

    /// Serialize the temporary data to account info
    fn write(&mut self) -> Result<(), ProgramError>;
}

/// Trait for any PDA internal data
pub trait PDAData {}

/// Token account to lock funds for vesting
#[derive(Default, Debug, PartialEq, Clone)]
pub struct Vault {
    pub amount: u64,
}

impl PDAData for Vault {}

impl PDAMethods<Vault> for PDA<'_, Vault> {
    fn size() -> usize {
        std::mem::size_of::<Account>()
    }

    fn check(&self) -> Result<(), ProgramError> {
        check_expected_address(self.info.key, self.program_id, seeds_convert!(self.seeds))
    }

    fn write(&mut self) -> Result<(), ProgramError> {
        Err(ProgramError::Custom(
            CustomError::WriteToPDAForbidden.into(),
        ))
    }
}

impl<'a> PDA<'a, Vault> {
    /// Create PDA structure object, validate seeds and pubkey
    pub fn new(
        program_id: &'a Pubkey,
        info: &'a AccountInfo<'a>,
        seed_key: &Pubkey,
    ) -> Result<PDA<'a, Vault>, ProgramError> {
        let pda = PDA {
            info,
            program_id,
            seeds: vec!["VAULT".as_bytes().to_vec(), seed_key.as_ref().to_vec()],
            data: Vault {
                amount: Account::unpack_from_slice(&info.data.borrow())
                    .unwrap_or_default()
                    .amount,
            },
        };
        pda.check()?;

        Ok(pda)
    }

    /// Create and init PDA
    pub fn create(
        &self,
        rent: &Rent,
        payer: &AccountInfo<'a>,
        mint: &AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        create_pda::<PDA<Vault>, Vault>(
            self.info,
            self.program_id,
            seeds_convert!(self.seeds),
            rent,
            payer,
            &spl_token::id(),
        )?;
        init_token_pda(self.info, mint, self.info.key)?;

        Ok(())
    }

    /// Transfer spl-token from Vault
    pub fn transfer_out(&self, wallet: &AccountInfo<'a>, amount: u64) -> Result<(), ProgramError> {
        transfer_from_pda(
            self.info,
            self.program_id,
            seeds_convert!(self.seeds),
            wallet,
            amount,
        )
    }
}

/// Token account to with beneficiary as authority
#[derive(Default, Debug, PartialEq, Clone)]
pub struct Distribute {}

impl PDAData for Distribute {}

impl PDAMethods<Distribute> for PDA<'_, Distribute> {
    fn size() -> usize {
        std::mem::size_of::<Account>()
    }

    fn check(&self) -> Result<(), ProgramError> {
        check_expected_address(self.info.key, self.program_id, seeds_convert!(self.seeds))
    }

    fn write(&mut self) -> Result<(), ProgramError> {
        Err(ProgramError::Custom(
            CustomError::WriteToPDAForbidden.into(),
        ))
    }
}

impl<'a> PDA<'a, Distribute> {
    /// Create PDA structure object, validate seeds and pubkey
    pub fn new(
        program_id: &'a Pubkey,
        info: &'a AccountInfo<'a>,
        seed_key: &Pubkey,
    ) -> Result<PDA<'a, Distribute>, ProgramError> {
        let pda = PDA {
            info,
            program_id,
            seeds: vec!["DISTRIBUTE".as_bytes().to_vec(), seed_key.as_ref().to_vec()],
            data: Distribute {},
        };
        pda.check()?;

        Ok(pda)
    }

    /// Create and init PDA
    pub fn create(
        &mut self,
        rent: &Rent,
        payer: &AccountInfo<'a>,
        mint: &AccountInfo<'a>,
        authority: &Pubkey,
    ) -> Result<(), ProgramError> {
        create_pda::<PDA<Distribute>, Distribute>(
            self.info,
            self.program_id,
            seeds_convert!(self.seeds),
            rent,
            payer,
            &spl_token::id(),
        )?;
        init_token_pda(self.info, mint, authority)?;

        Ok(())
    }
}

/// Data account to store vesting data
#[derive(BorshSerialize, BorshDeserialize, Default, Debug, PartialEq, Clone)]
pub struct Vesting {
    pub beneficiary: Pubkey,
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
        std::mem::size_of::<Vesting>()
    }

    fn check(&self) -> Result<(), ProgramError> {
        check_expected_address(self.info.key, self.program_id, seeds_convert!(self.seeds))
    }

    fn write(&mut self) -> Result<(), ProgramError> {
        self.data
            .serialize(&mut &mut self.info.data.borrow_mut()[..])
            .map_err(|x| ProgramError::BorshIoError(x.to_string()))
    }
}

impl<'a> PDA<'a, Vesting> {
    /// Create PDA structure object, validate seeds and pubkey
    pub fn new(
        program_id: &'a Pubkey,
        info: &'a AccountInfo<'a>,
        seed_key: &Pubkey,
    ) -> Result<PDA<'a, Vesting>, ProgramError> {
        let pda = PDA {
            info,
            program_id,
            seeds: vec!["VESTING".as_bytes().to_vec(), seed_key.as_ref().to_vec()],
            data: Vesting::try_from_slice(&info.data.borrow()).unwrap_or_default(),
        };
        pda.check()?;

        Ok(pda)
    }

    /// Create and init PDA
    pub fn create(&self, rent: &Rent, payer: &AccountInfo<'a>) -> Result<(), ProgramError> {
        create_pda::<PDA<Vesting>, Vesting>(
            self.info,
            self.program_id,
            seeds_convert!(self.seeds),
            rent,
            payer,
            self.program_id,
        )
    }
}

/// Sanity tests
#[cfg(test)]
mod test {
    use solana_program::program_pack::Pack;
    use solana_sdk::{account_info::AccountInfo, clock::Epoch, pubkey::Pubkey};
    use spl_token::state::Account;

    use super::{PDAMethods, Vault, Vesting, PDA};

    #[test]
    fn test_check_info() {
        let program_id = Pubkey::new_unique();
        let seed_key = Pubkey::new_unique();
        let lamports = &mut 0;

        let (vesting_key, _) =
            Pubkey::find_program_address(&["VESTING".as_bytes(), seed_key.as_ref()], &program_id);

        let mut data = vec![0; PDA::<Vesting>::size()];
        let info = AccountInfo::new(
            &vesting_key,
            false,
            false,
            lamports,
            &mut data,
            &program_id,
            false,
            Epoch::default(),
        );

        let vesting = &mut PDA::<Vesting>::new(&program_id, &info, &seed_key).unwrap();

        vesting.check().unwrap();
        vesting.seeds = vec!["VESTINK".as_bytes().to_vec(), seed_key.as_ref().to_vec()];
        vesting.check().unwrap_err();
    }

    #[test]
    fn test_read() {
        let program_id = Pubkey::new_unique();
        let seed_key = Pubkey::new_unique();
        let lamports = &mut 0;

        let (vault_key, _) =
            Pubkey::find_program_address(&["VAULT".as_bytes(), seed_key.as_ref()], &program_id);

        {
            let mut data = vec![0; PDA::<Vault>::size()];
            let info = AccountInfo::new(
                &vault_key,
                false,
                false,
                lamports,
                &mut data,
                &program_id,
                false,
                Epoch::default(),
            );

            let vault = &mut PDA::<Vault>::new(&program_id, &info, &seed_key).unwrap();

            assert_eq!(vault.data.amount, 0);
        }
        {
            let data = &mut vec![0; PDA::<Vault>::size()];
            Account {
                amount: 1010,
                ..Account::default()
            }
            .pack_into_slice(data);
            let info = AccountInfo::new(
                &vault_key,
                false,
                false,
                lamports,
                data,
                &program_id,
                false,
                Epoch::default(),
            );

            let vault = &mut PDA::<Vault>::new(&program_id, &info, &seed_key).unwrap();

            assert_eq!(vault.data.amount, 1010);
        }
    }

    #[test]
    fn test_write() {
        let program_id = Pubkey::new_unique();
        let seed_key = Pubkey::new_unique();
        let lamports = &mut 0;

        let (vesting_key, _) =
            Pubkey::find_program_address(&["VESTING".as_bytes(), seed_key.as_ref()], &program_id);

        let mut data = vec![0; PDA::<Vesting>::size()];
        let info = AccountInfo::new(
            &vesting_key,
            false,
            false,
            lamports,
            &mut data,
            &program_id,
            false,
            Epoch::default(),
        );

        let vesting = &mut PDA::<Vesting>::new(&program_id, &info, &seed_key).unwrap();
        vesting.data.amount = 1010;
        vesting.write().unwrap();

        let vesting_new = &mut PDA::<Vesting>::new(&program_id, vesting.info, &seed_key).unwrap();

        assert_eq!(vesting_new.data.amount, 1010);
    }
}
