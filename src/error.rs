/// Error enum definition
#[repr(u32)]
pub enum CustomError {
    InvalidPDAKey = 101,
    NotOwnedByTokenProgram = 102,
}

impl From<CustomError> for u32 {
    /// Convert error enum to u32
    fn from(error: CustomError) -> Self {
        error as u32
    }
}
