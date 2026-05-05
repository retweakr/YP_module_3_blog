use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[allow(dead_code)]
    #[error("user not found")]
    UserNotFound,
    #[error("user already exists")]
    UserAlreadyExists,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("post not found")]
    PostNotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("database error: {0}")]
    Database(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<sqlx::Error> for DomainError {
    fn from(e: sqlx::Error) -> Self {
        DomainError::Database(e.to_string())
    }
}
