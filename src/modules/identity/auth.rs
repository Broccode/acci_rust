use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;

use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
    traits::TenantAware,
};
use super::{
    models::{User, Credentials},
    repository::UserRepository,
    session::{Session, SessionStore, JwtConfig},
    rbac::RbacService,
    session_manager::SessionManager,
};

/// Service for handling user authentication
#[derive(Debug)]
pub struct AuthenticationService {
    repository: UserRepository,
    session_manager: SessionManager,
    rbac: RbacService,
}

impl AuthenticationService {
    /// Creates a new AuthenticationService instance
    pub fn new(
        repository: UserRepository,
        session_store: Box<dyn SessionStore>,
        jwt_config: JwtConfig,
        rbac: RbacService,
    ) -> Self {
        let session_manager = SessionManager::new(session_store, jwt_config);
        Self {
            repository,
            session_manager,
            rbac,
        }
    }

    /// Authenticates a user with their credentials
    pub async fn authenticate(&self, credentials: Credentials) -> Result<(User, Session)> {
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

        // Create session
        let session = self.session_manager.create_session(&user).await?;

        Ok((user, session))
    }

    /// Validates a session token
    pub async fn validate_session(&self, token: &str) -> Result<Session> {
        self.session_manager.validate_token(token).await
    }

    /// Refreshes a session
    pub async fn refresh_session(&self, session_id: uuid::Uuid) -> Result<Session> {
        self.session_manager.refresh_session(session_id).await
    }

    /// Logs out a user by removing their session
    pub async fn logout(&self, session_id: uuid::Uuid) -> Result<()> {
        self.session_manager.remove_session(session_id).await
    }

    /// Logs out all sessions for a user
    pub async fn logout_all(&self, user_id: UserId) -> Result<()> {
        self.session_manager.remove_user_sessions(user_id).await
    }
    
    /// Returns a reference to the RBAC service
    pub fn rbac(&self) -> &RbacService {
        &self.rbac
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
    use time::OffsetDateTime;

    #[tokio::test]
    async fn test_password_hashing() {
        let password = "test_password";
        let hash = AuthenticationService::hash_password(password).unwrap();

        // Verify the hash is valid
        let parsed_hash = PasswordHash::new(&hash).unwrap();
        assert!(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok());
    }

    // More tests will be added once we have the test database setup
}