use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

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

impl Error {
    /// Returns the appropriate HTTP status code for this error
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Authentication(_) => StatusCode::UNAUTHORIZED,
            Error::Authorization(_) => StatusCode::FORBIDDEN,
            Error::Validation(_) => StatusCode::BAD_REQUEST,
            Error::Tenant(_) => StatusCode::BAD_REQUEST,
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(json!({
            "error": {
                "message": self.to_string(),
                "code": status.as_u16(),
                "correlation_id": Uuid::new_v4(),
            }
        }));

        (status, body).into_response()
    }
}

/// Result type alias using our Error type
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            Error::Authentication("test".into()).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            Error::NotFound("test".into()).status_code(),
            StatusCode::NOT_FOUND
        );
    }
}