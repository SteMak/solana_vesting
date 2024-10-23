/// Error enum definition
#[repr(u32)]
pub enum CustomError {
    InvalidPDAKey = 101,
    ZeroAmount,
    CliffOverDuration,
    StartCliffOverflow,
    WriteToPDAForbidden,
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
    use coverage_helper::test;

    use solana_sdk::program_error::ProgramError;

    use crate::error::CustomError;

    #[test]
    fn test_convert_error() {
        assert!(ProgramError::from(101) == ProgramError::Custom(CustomError::InvalidPDAKey.into()));
        assert!(ProgramError::from(102) == ProgramError::Custom(CustomError::ZeroAmount.into()));
    }
}
