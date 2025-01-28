use thiserror::Error;

/// Common error types for the ACCI Framework
#[derive(Debug, Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Authorization failed: {0}")]
    Authorization(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Tenant error: {0}")]
    Tenant(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias using our Error type
pub type Result<T> = std::result::Result<T, Error>;