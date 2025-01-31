use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{
    mfa::MfaService,
    models::{Credentials, Role, RoleType, User},
    repository::UserRepository,
    session::{Session, SessionStore},
};
use crate::{
    modules::tenant::models::Tenant,
    shared::{
        error::{Error, Result},
        types::{TenantId, UserId},
    },
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

    /// Authenticates a user with MFA
    pub async fn authenticate_with_mfa(
        &self,
        credentials: Credentials,
        mfa_code: String,
    ) -> Result<Session> {
        let user = self
            .repository
            .get_user_by_email(&credentials.email, credentials.tenant_id)
            .await?
            .ok_or_else(|| Error::Authentication("Invalid credentials".to_string()))?;

        if !Self::verify_password(&credentials.password, &user.password_hash)? {
            return Err(Error::Authentication("Invalid credentials".to_string()));
        }

        if !user.mfa_enabled {
            return Err(Error::Authentication(
                "MFA not enabled for this user".to_string(),
            ));
        }

        let mfa_secret = user
            .mfa_secret
            .as_ref()
            .ok_or_else(|| Error::Internal("MFA secret not found".to_string()))?;

        if !self.mfa_service.verify_code(mfa_secret, &mfa_code)? {
            return Err(Error::Authentication("Invalid MFA code".to_string()));
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
    use crate::modules::identity::mfa::{MfaConfig, MfaService};
    use std::collections::HashMap;
    use std::sync::Mutex;

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
        let service = AuthenticationService::new(repository, session_store);

        // Create test tenant
        let tenant = Tenant::new(
            "Test Tenant".to_string(),
            format!("{}.example.com", Uuid::new_v4()),
        );

        let mut retries = 3;
        while retries > 0 {
            match sqlx::query!(
                r#"INSERT INTO tenants (id, name, domain, active) VALUES ($1, $2, $3, $4)"#,
                tenant.id.0 as uuid::Uuid,
                tenant.name,
                tenant.domain,
                tenant.active
            )
            .execute(&db.get_pool())
            .await
            {
                Ok(_) => break,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to create tenant: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        }

        // Test user registration
        let credentials = Credentials {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            tenant_id: tenant.id,
            mfa_code: None,
        };

        let mut retries = 3;
        let user = loop {
            match service.register_user(credentials.clone()).await {
                Ok(u) => break u,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to register user: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };

        assert_eq!(user.email, credentials.email);
        assert_eq!(user.tenant_id, credentials.tenant_id);

        // Test authentication
        let mut retries = 3;
        let session = loop {
            match service.authenticate(credentials.clone()).await {
                Ok(s) => break s,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to authenticate: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };

        assert_eq!(session.user_id, user.id);
        assert_eq!(session.tenant_id, user.tenant_id);
    }

    #[tokio::test]
    async fn test_mfa_authentication() {
        let (db, _container) = create_test_db().await.unwrap();
        let repository = UserRepository::new(db.get_pool());
        let session_store = Box::new(MockSessionStore::default());
        let service = AuthenticationService::new(repository, session_store);

        // Create test tenant
        let tenant = Tenant::new(
            "Test Tenant".to_string(),
            format!("{}.example.com", Uuid::new_v4()),
        );

        let mut retries = 3;
        while retries > 0 {
            match sqlx::query!(
                r#"INSERT INTO tenants (id, name, domain, active) VALUES ($1, $2, $3, $4)"#,
                tenant.id.0 as uuid::Uuid,
                tenant.name,
                tenant.domain,
                tenant.active
            )
            .execute(&db.get_pool())
            .await
            {
                Ok(_) => break,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to create tenant: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        }

        // Test user registration
        let credentials = Credentials {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            tenant_id: tenant.id,
            mfa_code: None,
        };

        let mut retries = 3;
        let user = loop {
            match service.register_user(credentials.clone()).await {
                Ok(u) => break u,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to register user: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };

        assert_eq!(user.email, credentials.email);
        assert_eq!(user.tenant_id, credentials.tenant_id);

        // Enable MFA
        let mfa_config = MfaConfig::default();
        let mfa_service = MfaService::new(mfa_config);
        let secret = mfa_service.generate_secret().unwrap();

        // Update user with MFA enabled
        let mut user = user;
        user.mfa_enabled = true;
        user.mfa_secret = Some(secret.clone());

        let mut retries = 3;
        while retries > 0 {
            match sqlx::query!(
                r#"
                UPDATE users 
                SET mfa_enabled = $1, mfa_secret = $2 
                WHERE id = $3
                "#,
                user.mfa_enabled,
                user.mfa_secret,
                user.id.0 as uuid::Uuid
            )
            .execute(&db.get_pool())
            .await
            {
                Ok(_) => break,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to update user: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        }

        // Generate valid TOTP code
        let totp = mfa_service.create_totp(&secret).unwrap();
        let code = totp.generate_current().unwrap();

        // Test MFA authentication
        let mut retries = 3;
        let session = loop {
            match service
                .authenticate_with_mfa(credentials.clone(), code.clone())
                .await
            {
                Ok(s) => break s,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to authenticate with MFA: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };

        assert_eq!(session.user_id, user.id);
        assert_eq!(session.tenant_id, user.tenant_id);
    }
}
