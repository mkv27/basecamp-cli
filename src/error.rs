use std::error::Error;
use std::fmt::{Display, Formatter};

pub type AppResult<T> = Result<T, AppError>;

pub const OAUTH_UNAUTHORIZED_MESSAGE: &str = "Basecamp rejected access token (401 Unauthorized).";
pub const OAUTH_UNAUTHORIZED_RELOGIN_MESSAGE: &str =
    "Basecamp rejected access token (401 Unauthorized). Run `basecamp-cli login` again.";
pub const OAUTH_FORBIDDEN_MESSAGE: &str = "Basecamp denied access (403 Forbidden).";

#[derive(Debug, Clone, Copy)]
pub struct OAuthStatusMessages<'a> {
    pub unauthorized: &'a str,
    pub forbidden: &'a str,
}

impl<'a> OAuthStatusMessages<'a> {
    pub const fn new(unauthorized: &'a str, forbidden: &'a str) -> Self {
        Self {
            unauthorized,
            forbidden,
        }
    }
}

pub fn oauth_error_from_status(
    status_code: u16,
    messages: OAuthStatusMessages<'_>,
) -> Option<AppError> {
    match status_code {
        401 => Some(AppError::oauth(messages.unauthorized)),
        403 => Some(AppError::oauth(messages.forbidden)),
        _ => None,
    }
}

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
