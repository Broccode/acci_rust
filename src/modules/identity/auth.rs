use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
    traits::TenantAware,
};
use super::{
    models::{Credentials, User},
    repository::UserRepository,
};

/// Service for handling authentication-related operations
#[derive(Debug, Clone)]
pub struct AuthenticationService {
    repository: UserRepository,
}

impl AuthenticationService {
    /// Creates a new AuthenticationService instance
    pub fn new(repository: UserRepository) -> Self {
        Self { repository }
    }

    /// Authenticates a user with the provided credentials
    pub async fn authenticate(&self, credentials: Credentials) -> Result<User> {
        // Set tenant context for the repository
        self.repository
            .set_tenant_context(credentials.tenant_id)
            .await?;

        // Find user by email
        let user = self
            .repository
            .get_user_by_email(&credentials.email, credentials.tenant_id)
            .await?;

        // Verify password
        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| Error::Authentication(e.to_string()))?;

        Argon2::default()
            .verify_password(credentials.password.as_bytes(), &parsed_hash)
            .map_err(|_| Error::Authentication("Invalid password".into()))?;

        // Update last login timestamp
        self.repository.update_last_login(user.id).await?;

        // Clear tenant context
        self.repository.clear_tenant_context().await?;

        Ok(user)
    }

    /// Creates a new user with the provided credentials
    pub async fn create_user(
        &self,
        email: String,
        password: String,
        tenant_id: TenantId,
    ) -> Result<User> {
        // Set tenant context
        self.repository.set_tenant_context(tenant_id).await?;

        // Hash password
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| Error::Internal(e.to_string()))?
            .to_string();

        let now = OffsetDateTime::now_utc();
        let user = User {
            id: UserId(Uuid::new_v4()),
            tenant_id,
            email,
            password_hash,
            roles: vec![], // Default roles could be added here
            active: true,
            last_login: None,
            created_at: now,
            updated_at: now,
        };

        // Create user in database
        let created_user = self.repository.create_user(&user).await?;

        // Clear tenant context
        self.repository.clear_tenant_context().await?;

        Ok(created_user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Implement tests
    // Will need to set up test database and migrations first
}