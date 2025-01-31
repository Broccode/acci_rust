use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

/// Result type for the application
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for the application
#[derive(Debug, Error)]
pub enum Error {
    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Authorization error
    #[error("Authorization error: {0}")]
    Authorization(String),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid input error
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg),
            Error::Authorization(msg) => (StatusCode::FORBIDDEN, msg),
            Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Error::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        (status, message).into_response()
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound("Record not found".to_string()),
            _ => Self::Database(err.to_string()),
        }
    }
}

impl From<redis::RedisError> for Error {
    fn from(err: redis::RedisError) -> Self {
        Self::Database(format!("Redis error: {}", err))
    }
}

impl From<jsonwebtoken::errors::Error> for Error {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Self::Authentication(format!("JWT error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let db_error = sqlx::Error::RowNotFound;
        let error: Error = db_error.into();
        assert!(matches!(error, Error::NotFound(_)));

        let redis_error = redis::RedisError::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test error",
        ));
        let error: Error = redis_error.into();
        assert!(matches!(error, Error::Database(_)));

        let jwt_error = jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        );
        let error: Error = jwt_error.into();
        assert!(matches!(error, Error::Authentication(_)));
    }

    #[test]
    fn test_error_display() {
        let error = Error::Database("test error".to_string());
        assert_eq!(error.to_string(), "Database error: test error");

        let error = Error::Authentication("test error".to_string());
        assert_eq!(error.to_string(), "Authentication error: test error");

        let error = Error::Authorization("test error".to_string());
        assert_eq!(error.to_string(), "Authorization error: test error");

        let error = Error::NotFound("test error".to_string());
        assert_eq!(error.to_string(), "Not found: test error");

        let error = Error::InvalidInput("test error".to_string());
        assert_eq!(error.to_string(), "Invalid input: test error");

        let error = Error::Internal("test error".to_string());
        assert_eq!(error.to_string(), "Internal error: test error");

        let error = Error::Validation("test error".to_string());
        assert_eq!(error.to_string(), "Validation error: test error");
    }

    #[test]
    fn test_error_response() {
        let error = Error::NotFound("test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let error = Error::Authentication("test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let error = Error::Authorization("test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let error = Error::InvalidInput("test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let error = Error::Internal("test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let error = Error::Database("test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let error = Error::Validation("test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}