use std::error::Error;
use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, PartialEq)]
pub enum EngineError {
    NoValidQueueFamily,
    OutOfBounds,
    ZeroSized,
    CompileError(String),
    WrongSpecialization,
}

impl EngineError {
    pub fn into_result<T>(self) -> Result<T> {
        Err(self.into())
    }
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for EngineError {}

// impl<T> Into<Result<T, EngineError>> for EngineError {
//     fn into(self) -> Result<T, EngineError> {
//         Err(self)
//     }
// }
