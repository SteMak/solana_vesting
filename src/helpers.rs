use solana_program::{
    account_info::AccountInfo,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};

use crate::error::CustomError;

/// Sized accounts interface
pub trait PDA {
    fn size() -> usize;
}

/// Create PDA using given parameters
pub fn create_pda<'a, T: PDA>(
    pda: &AccountInfo<'a>,
    program_id: &Pubkey,
    pda_seeds: &[&[u8]],
    rent: &Rent,
    payer: &AccountInfo<'a>,
    owner: &Pubkey,
) -> Result<(), ProgramError> {
    // `CreateAccount` instruction requires `payer` to be writable signer
    assert!(payer.is_signer);
    assert!(payer.is_writable);
    // `CreateAccount` instruction requires `pda` to be writable and signer (invoke_signed)
    assert!(pda.is_writable);

    // Get `bump` seed and check `pda` corresponds seeds
    let (calculated_key, bump) = Pubkey::find_program_address(pda_seeds, program_id);
    assert!(*pda.key == calculated_key);

    // Get balance for rent exemption
    let space = T::size();
    let lamports = rent.minimum_balance(space);

    // Invoke `CreateAccount`, instruction requires `pda` to be signer
    invoke_signed(
        &system_instruction::create_account(payer.key, pda.key, lamports, space as u64, owner),
        &[payer.clone(), pda.clone()],
        &[pda_seeds, &[&[bump]]],
    )?;

    Ok(())
}

/// Check PDA corresponds seeds
pub fn check_expected_address(
    pda: &AccountInfo,
    program_id: &Pubkey,
    pda_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    // Get PDA from seeds and compare
    let (calculated_key, _) = Pubkey::find_program_address(pda_seeds, program_id);
    if *pda.key != calculated_key {
        return Err(ProgramError::Custom(CustomError::InvalidPDAKey.into()));
    }

    Ok(())
}

/// Initialize PDA with token account
pub fn init_token_pda<'a>(
    pda: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
) -> Result<(), ProgramError> {
    // `InitializeAccount3` instruction requires `pda` to be writable
    assert!(pda.is_writable);

    // Sanity token account ownership check
    assert!(*pda.owner == spl_token::id());

    // Invoke `InitializeAccount3`, instruction requires `mint` to be provided
    invoke(
        &spl_token::instruction::initialize_account3(
            &spl_token::id(),
            pda.key,
            mint.key,
            &spl_token::id(),
        )?,
        &[pda.clone(), mint.clone()],
    )?;

    Ok(())
}

/// Transfer spl-token to PDA
pub fn transfer_to_pda<'a>(
    pda: &AccountInfo<'a>,
    wallet: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
) -> Result<(), ProgramError> {
    // `Transfer` instruction requires `authority` to be signer
    assert!(authority.is_signer);
    // `Transfer` instruction requires `wallet` and `pda` to be writable
    assert!(wallet.is_writable);
    assert!(pda.is_writable);

    // Sanity token account ownership checks
    assert!(*pda.owner == spl_token::id());
    assert!(*wallet.owner == spl_token::id());

    // Invoke `Transfer`
    invoke(
        &spl_token::instruction::transfer(
            &spl_token::id(),
            wallet.key,
            pda.key,
            &spl_token::id(),
            &[pda.key],
            amount,
        )?,
        &[wallet.clone(), pda.clone(), authority.clone()],
    )?;

    Ok(())
}

/// Transfer spl-token from PDA
pub fn transfer_from_pda<'a>(
    pda: &AccountInfo<'a>,
    program_id: &Pubkey,
    pda_seeds: &[&[u8]],
    wallet: &AccountInfo<'a>,
    amount: u64,
) -> Result<(), ProgramError> {
    // `Transfer` instruction requires `pda` to be writable and signer (invoke_signed)
    assert!(pda.is_writable);
    // `Transfer` instruction requires `wallet` to be writable
    assert!(wallet.is_writable);

    // Sanity token account ownership checks
    assert!(*pda.owner == spl_token::id());
    assert!(*wallet.owner == spl_token::id());

    // Get `bump` seed and check `pda` corresponds seeds
    let (calculated_key, bump) = Pubkey::find_program_address(pda_seeds, program_id);
    assert!(*pda.key == calculated_key);

    // Invoke `Transfer`, instruction requires `pda` to be signer
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
    )?;

    Ok(())
}
