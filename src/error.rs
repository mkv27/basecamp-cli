use std::error::Error;
use std::fmt::{Display, Formatter};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone)]
pub struct AppError {
    pub code: i32,
    pub message: String,
}

impl AppError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn generic(message: impl Into<String>) -> Self {
        Self::new(1, message)
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(2, message)
    }

    pub fn oauth(message: impl Into<String>) -> Self {
        Self::new(3, message)
    }

    pub fn no_account(message: impl Into<String>) -> Self {
        Self::new(4, message)
    }

    pub fn secure_storage(message: impl Into<String>) -> Self {
        Self::new(5, message)
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for AppError {}
