use std::fmt::{Debug, Display};

pub type Result<T> = core::result::Result<T, Error>;

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

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Error")
            .field("message", &self.message)
            .finish()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<path_abs::Error> for Error {
    fn from(path: path_abs::Error) -> Self {
        Error { message: path.to_string() }
    }
}
