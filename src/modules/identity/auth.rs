use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
use time::OffsetDateTime;

use super::{
    mfa::MfaService,
    models::{Credentials, User},
    repository::UserRepository,
    session::{Session, SessionStore},
};
use crate::shared::{
    error::{Error, Result},
    types::UserId,
};

/// Authentication service for handling user authentication
#[derive(Debug)]
pub struct AuthenticationService {
    repository: UserRepository,
    session_store: Box<dyn SessionStore>,
    mfa_service: MfaService,
}

impl AuthenticationService {
    /// Creates a new AuthenticationService instance
    pub fn new(repository: UserRepository, session_store: Box<dyn SessionStore>) -> Self {
        Self {
            repository,
            session_store,
            mfa_service: MfaService::new(Default::default()),
        }
    }

    /// Registers a new user
    pub async fn register_user(&self, credentials: Credentials) -> Result<User> {
        let password_hash = Self::hash_password(&credentials.password)?;
        let user = User {
            id: UserId::new(),
            tenant_id: credentials.tenant_id,
            email: credentials.email,
            password_hash,
            active: true,
            roles: vec![],
            last_login: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            mfa_enabled: false,
            mfa_secret: None,
        };

        self.repository.create_user(user).await
    }

    /// Authenticates a user with credentials
    pub async fn authenticate(&self, credentials: Credentials) -> Result<Session> {
        let user = self
            .repository
            .get_user_by_email(&credentials.email, credentials.tenant_id)
            .await?
            .ok_or_else(|| Error::Authentication("Invalid credentials".to_string()))?;

        if !Self::verify_password(&credentials.password, &user.password_hash)? {
            return Err(Error::Authentication("Invalid credentials".to_string()));
        }

        // Verify MFA if enabled
        if user.mfa_enabled {
            let mfa_code = credentials
                .mfa_code
                .ok_or_else(|| Error::Authentication("MFA code required".to_string()))?;

            if !self.mfa_service.verify_code(
                user.mfa_secret
                    .as_ref()
                    .ok_or_else(|| Error::Internal("MFA secret not found".to_string()))?,
                &mfa_code,
            )? {
                return Err(Error::Authentication("Invalid MFA code".to_string()));
            }
        }

        self.repository.update_last_login(user.id).await?;

        let session = Session::new(
            user.id,
            user.tenant_id,
            "".to_string(),
            time::Duration::hours(1),
        );

        self.session_store.store_session(&session).await?;

        Ok(session)
    }

    /// Hashes a password using Argon2
    pub fn hash_password(password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| Error::Internal(format!("Failed to hash password: {}", e)))?
            .to_string();
        Ok(password_hash)
    }

    /// Verifies a password against a hash
    fn verify_password(password: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| Error::Internal(format!("Failed to parse password hash: {}", e)))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::database::tests::create_test_db;
    use crate::modules::tenant::models::Tenant;

    #[derive(Debug, Default)]
    struct MockSessionStore {
        sessions: Mutex<HashMap<String, Session>>,
    }

    #[async_trait::async_trait]
    impl SessionStore for MockSessionStore {
        async fn store_session(&self, session: &Session) -> Result<()> {
            self.sessions
                .lock()
                .unwrap()
                .insert(session.token.clone(), session.clone());
            Ok(())
        }

        async fn get_session(&self, _id: Uuid) -> Result<Option<Session>> {
            Ok(None)
        }

        async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>> {
            Ok(self.sessions.lock().unwrap().get(token).cloned())
        }

        async fn remove_session(&self, _id: Uuid) -> Result<()> {
            Ok(())
        }

        async fn remove_user_sessions(&self, _user_id: UserId) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_authentication() {
        let (db, _container) = create_test_db().await.unwrap();
        let repository = UserRepository::new(db.get_pool());
        let session_store = Box::new(MockSessionStore::default());
        let auth_service = AuthenticationService::new(repository, session_store);

        // Create test tenant
        let tenant = Tenant::new("Test Tenant".to_string());
        sqlx::query!(
            "INSERT INTO tenants (id, name) VALUES ($1, $2)",
            tenant.id.0,
            tenant.name
        )
        .execute(auth_service.repository.get_pool())
        .await
        .unwrap();

        // Test user registration
        let credentials = Credentials {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            tenant_id: tenant.id,
            mfa_code: None,
        };

        let user = auth_service
            .register_user(credentials.clone())
            .await
            .unwrap();

        assert_eq!(user.email, credentials.email);
        assert!(user.active);

        // Test authentication
        let session = auth_service.authenticate(credentials).await.unwrap();

        assert_eq!(session.user_id, user.id);
        assert!(session.expires_at > OffsetDateTime::now_utc());
    }

    #[tokio::test]
    async fn test_mfa_authentication() {
        let (db, _container) = create_test_db().await.unwrap();
        let repository = UserRepository::new(db.get_pool());
        let session_store = Box::new(MockSessionStore::default());
        let auth_service = AuthenticationService::new(repository, session_store);

        // Create test tenant
        let tenant = Tenant::new("Test Tenant".to_string());
        sqlx::query!(
            "INSERT INTO tenants (id, name) VALUES ($1, $2)",
            tenant.id.0,
            tenant.name
        )
        .execute(auth_service.repository.get_pool())
        .await
        .unwrap();

        // Create test user with MFA enabled
        let secret = auth_service.mfa_service.generate_secret().unwrap();
        let user = User {
            id: UserId::new(),
            tenant_id: tenant.id,
            email: "test@example.com".to_string(),
            password_hash: AuthenticationService::hash_password("password123").unwrap(),
            roles: vec![],
            active: true,
            last_login: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            mfa_enabled: true,
            mfa_secret: Some(secret.clone()),
        };

        let created_user = auth_service.repository.create_user(user).await.unwrap();

        // Generate valid TOTP code
        let totp = auth_service.mfa_service.create_totp(&secret).unwrap();
        let code = totp.generate_current().unwrap();

        // Test valid credentials with MFA
        let credentials = Credentials {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            tenant_id: created_user.tenant_id,
            mfa_code: Some(code),
        };

        let session = auth_service.authenticate(credentials).await.unwrap();
        assert_eq!(session.user_id, created_user.id);

        // Test invalid MFA code
        let credentials = Credentials {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            tenant_id: created_user.tenant_id,
            mfa_code: Some("000000".to_string()),
        };

        assert!(auth_service.authenticate(credentials).await.is_err());

        // Test missing MFA code
        let credentials = Credentials {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            tenant_id: created_user.tenant_id,
            mfa_code: None,
        };

        assert!(auth_service.authenticate(credentials).await.is_err());
    }
}
