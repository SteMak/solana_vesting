use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program::invoke_signed, program_error::ProgramError, pubkey::Pubkey,
    rent::Rent, system_instruction, system_program, sysvar::Sysvar,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct State {
    pub owner: Pubkey,
    pub token_mint: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Vesting {
    pub duration: u64,
    pub amount: u64,
    pub cliff: u64,
    pub start_date: u64,

    pub claimed: u64,
}

const STATE_SEED: &[u8] = "STATE".as_bytes();
const VESTING_SEED: &[u8] = "VESTING".as_bytes();

impl State {
    pub fn check_info(info: &AccountInfo, program_id: &Pubkey) -> Result<(), ProgramError> {
        check(info, program_id, &[STATE_SEED])
    }

    pub fn create<'a>(
        program_id: &Pubkey,
        payer: &AccountInfo<'a>,
        pda: &AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        create_pda::<State>(program_id, payer, pda, &[STATE_SEED])
    }

    pub fn get_data(info: &AccountInfo) -> Result<State, ProgramError> {
        let data = State::try_from_slice(&info.try_borrow_data()?)?;

        Ok(data)
    }

    pub fn set_data(self, info: &AccountInfo) -> Result<(), ProgramError> {
        self.serialize(&mut *info.try_borrow_mut_data()?)?;

        Ok(())
    }
}

impl Vesting {
    pub fn check_info(
        info: &AccountInfo,
        program_id: &Pubkey,
        user: Pubkey,
    ) -> Result<(), ProgramError> {
        check(info, program_id, &[VESTING_SEED, &user.to_bytes()])
    }

    pub fn create<'a>(
        program_id: &Pubkey,
        payer: &AccountInfo<'a>,
        pda: &AccountInfo<'a>,
        user: Pubkey,
    ) -> Result<(), ProgramError> {
        create_pda::<Vesting>(program_id, payer, pda, &[VESTING_SEED, &user.to_bytes()])
    }

    pub fn get_data(info: &AccountInfo) -> Result<Vesting, ProgramError> {
        let data = Vesting::try_from_slice(&info.try_borrow_data()?)?;

        Ok(data)
    }

    pub fn set_data(self, info: &AccountInfo) -> Result<(), ProgramError> {
        self.serialize(&mut *info.try_borrow_mut_data()?)?;

        Ok(())
    }
}

fn check(info: &AccountInfo, program_id: &Pubkey, seeds: &[&[u8]]) -> Result<(), ProgramError> {
    let (calculated_key, _) = Pubkey::find_program_address(seeds, &program_id);
    if *info.key != calculated_key {
        return Err(ProgramError::InvalidAccountOwner);
    }

    Ok(())
}

fn create_pda<'a, PDAData>(
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let (_, bump) = Pubkey::find_program_address(seeds, &program_id);
    let rent = Rent::get()?;

    let space: usize = std::mem::size_of::<PDAData>();
    let lamports = rent.minimum_balance(space);

    // TODO check that it fails if seeds inconsistent with info
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            pda.key,
            lamports,
            space as u64,
            &system_program::ID,
        ),
        &[payer.clone(), pda.clone()],
        &[seeds, &[&[bump]]],
    )?;

    Ok(())
}
