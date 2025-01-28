use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
use time::OffsetDateTime;

use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
    traits::TenantAware,
};
use super::{
    models::{User, Credentials},
    repository::UserRepository,
};

/// Service for handling user authentication
#[derive(Debug)]
pub struct AuthenticationService {
    repository: UserRepository,
}

impl AuthenticationService {
    /// Creates a new AuthenticationService instance
    pub fn new(repository: UserRepository) -> Self {
        Self { repository }
    }

    /// Authenticates a user with their credentials
    pub async fn authenticate(&self, credentials: Credentials) -> Result<User> {
        // Set tenant context for the repository
        self.repository.set_tenant_context(credentials.tenant_id).await?;

        // Get user by email
        let user = self.repository
            .get_user_by_email(&credentials.email, credentials.tenant_id)
            .await?;

        // Clear tenant context after use
        self.repository.clear_tenant_context().await?;

        // Verify password
        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| Error::Internal(format!("Failed to parse password hash: {}", e)))?;

        Argon2::default()
            .verify_password(
                credentials.password.as_bytes(),
                &parsed_hash,
            )
            .map_err(|_| Error::Authentication("Invalid password".to_string()))?;

        // Update last login timestamp
        self.repository.update_last_login(user.id).await?;

        Ok(user)
    }

    /// Hashes a password using Argon2
    pub fn hash_password(password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| Error::Internal(format!("Failed to hash password: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Implement tests
    // Will need to set up test database and migrations first
}