use std::{
    fmt::{Debug, Display},
    io,
};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: &'static str) -> Self {
        Error {
            message: String::from(message),
        }
    }

    pub fn from_string(message: String) -> Self {
        Error { message }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error {
            message: value.to_string(),
        }
    }
}
