use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use spl_token::state::Account;

#[derive(Debug)]
#[repr(u32)]
pub enum PDAError {
    InvalidPDAKey = 201,
}

impl From<PDAError> for u32 {
    fn from(error: PDAError) -> Self {
        error as u32
    }
}

trait PDA {
    fn size() -> usize;
}

pub struct Vault;

const VAULT_SEED: &[u8] = "VAULT".as_bytes();

impl PDA for Vault {
    fn size() -> usize {
        return std::mem::size_of::<Account>();
    }
}

impl Vault {
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
            &spl_token::id(),
            payer,
            &[
                VAULT_SEED,
                &mint.key.to_bytes(),
                &user.to_bytes(),
                &nonce.to_le_bytes(),
            ],
        )?;
        init_token_pda(pda, mint)?;

        Ok(())
    }

    pub fn transfer_in<'a>(
        pda: &AccountInfo<'a>,
        wallet: &AccountInfo<'a>,
        signer: &AccountInfo<'a>,
        amount: u64,
    ) -> Result<(), ProgramError> {
        transfer_token_to_pda(pda, wallet, signer, amount)
    }

    pub fn transfer_out<'a>(
        pda: &AccountInfo<'a>,
        program_id: &Pubkey,
        wallet: &AccountInfo<'a>,
        mint: &Pubkey,
        user: Pubkey,
        nonce: u64,
        amount: u64,
    ) -> Result<(), ProgramError> {
        transfer_token_from_pda(
            pda,
            program_id,
            wallet,
            amount,
            &[
                VAULT_SEED,
                &mint.to_bytes(),
                &user.to_bytes(),
                &nonce.to_le_bytes(),
            ],
        )
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Vesting {
    pub amount: u64,
    pub claimed: u64,

    pub start: u64,
    pub cliff: u64,
    pub duration: u64,
}

const VESTING_SEED: &[u8] = "VESTING".as_bytes();

impl PDA for Vesting {
    fn size() -> usize {
        return std::mem::size_of::<Vesting>();
    }
}

impl Vesting {
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
            program_id,
            payer,
            &[
                VESTING_SEED,
                &mint.to_bytes(),
                &user.to_bytes(),
                &nonce.to_le_bytes(),
            ],
        )
    }

    pub fn get_data(info: &AccountInfo) -> Result<Vesting, ProgramError> {
        let data = Vesting::try_from_slice(&info.try_borrow_data()?)
            .map_err(|x| ProgramError::BorshIoError(x.to_string()))?;

        Ok(data)
    }

    pub fn set_data(self, info: &AccountInfo) -> Result<(), ProgramError> {
        self.serialize(&mut *info.try_borrow_mut_data()?)
            .map_err(|x| ProgramError::BorshIoError(x.to_string()))?;

        Ok(())
    }
}

fn check_expected_address(
    pda: &AccountInfo,
    program_id: &Pubkey,
    pda_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let (calculated_key, _) = Pubkey::find_program_address(pda_seeds, &program_id);
    if *pda.key != calculated_key {
        return Err(ProgramError::Custom(PDAError::InvalidPDAKey.into()));
    }

    Ok(())
}

fn create_pda<'a, T: PDA>(
    pda: &AccountInfo<'a>,
    program_id: &Pubkey,
    owner: &Pubkey,
    payer: &AccountInfo<'a>,
    pda_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let (calculated_key, bump) = Pubkey::find_program_address(pda_seeds, &program_id);
    if *pda.key != calculated_key {
        return Err(ProgramError::Custom(PDAError::InvalidPDAKey.into()));
    }

    let rent = Rent::get()?;

    let space = T::size();
    let lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(payer.key, pda.key, lamports, space as u64, owner),
        &[payer.clone(), pda.clone()],
        &[pda_seeds, &[&[bump]]],
    )?;

    Ok(())
}

fn init_token_pda<'a>(pda: &AccountInfo<'a>, mint: &AccountInfo<'a>) -> Result<(), ProgramError> {
    invoke(
        &spl_token::instruction::initialize_account3(
            &spl_token::id(),
            pda.key,
            mint.key,
            &spl_token::id(),
        )?,
        &[pda.clone(), mint.clone()],
    )
}

fn transfer_token_from_pda<'a>(
    pda: &AccountInfo<'a>,
    program_id: &Pubkey,
    wallet: &AccountInfo<'a>,
    amount: u64,
    pda_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let (calculated_key, bump) = Pubkey::find_program_address(pda_seeds, &program_id);
    if *pda.key != calculated_key {
        return Err(ProgramError::Custom(PDAError::InvalidPDAKey.into()));
    }

    invoke_signed(
        &spl_token::instruction::transfer(
            &spl_token::id(),
            pda.key,
            wallet.key,
            &spl_token::id(),
            &[pda.key],
            amount,
        )?,
        &[pda.clone(), wallet.clone(), pda.clone()],
        &[pda_seeds, &[&[bump]]],
    )
}

fn transfer_token_to_pda<'a>(
    pda: &AccountInfo<'a>,
    wallet: &AccountInfo<'a>,
    signer: &AccountInfo<'a>,
    amount: u64,
) -> Result<(), ProgramError> {
    invoke(
        &spl_token::instruction::transfer(
            &spl_token::id(),
            wallet.key,
            pda.key,
            &spl_token::id(),
            &[pda.key],
            amount,
        )?,
        &[wallet.clone(), pda.clone(), signer.clone()],
    )
}
