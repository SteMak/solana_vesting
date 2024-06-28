/// Error enum definition
#[repr(u32)]
pub enum CustomError {
    InvalidPDAKey = 101,
    NotOwnedByTokenProgram = 102,
    UnauthorizedClaimer = 103,
    ZeroAmount = 104,
    CliffOverDuration = 105,
    StartCliffOverflow = 106,
}

impl From<CustomError> for u32 {
    /// Convert error enum to u32
    fn from(error: CustomError) -> Self {
        error as u32
    }
}

/// Sanity tests
#[cfg(test)]
mod test {
    use super::CustomError;

    use solana_sdk::program_error::ProgramError;

    #[test]
    fn test_convert_error() {
        assert!(ProgramError::from(101) == ProgramError::Custom(CustomError::InvalidPDAKey.into()));
    }
}
