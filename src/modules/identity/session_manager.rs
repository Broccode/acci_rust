use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use time::Duration;
use uuid::Uuid;

use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
};
use super::{
    models::User,
    session::{Session, SessionStore, JwtConfig, Claims},
};

/// Manages user sessions and JWT tokens
pub struct SessionManager {
    store: Box<dyn SessionStore>,
    jwt_config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl std::fmt::Debug for SessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionManager")
            // Skip fields that don't implement Debug
            .finish_non_exhaustive()
    }
}

impl SessionManager {
    /// Creates a new SessionManager instance
    pub fn new(store: Box<dyn SessionStore>, jwt_config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(jwt_config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(jwt_config.secret.as_bytes());
        Self {
            store,
            jwt_config,
            encoding_key,
            decoding_key,
        }
    }

    /// Creates a new session for a user
    pub async fn create_session(&self, user: &User) -> Result<Session> {
        // Generate session ID
        let session_id = Uuid::new_v4();

        // Create JWT claims
        let claims = Claims::new(
            user.id,
            user.tenant_id,
            &self.jwt_config,
            session_id,
        );

        // Generate JWT token
        let token = encode(
            &Header::default(),
            &claims,
            &self.encoding_key,
        ).map_err(|e| Error::Internal(format!("Failed to create JWT token: {}", e)))?;

        // Create session
        let session = Session::new(
            user.id,
            user.tenant_id,
            token,
            self.jwt_config.expiration,
        );

        // Store session
        self.store.store_session(&session).await?;

        Ok(session)
    }

    /// Validates a JWT token and returns the associated session
    pub async fn validate_token(&self, token: &str) -> Result<Session> {
        // First check if session exists
        let session = self.store
            .get_session_by_token(token)
            .await?
            .ok_or_else(|| Error::Authentication("Invalid session".to_string()))?;

        // Check if session is expired
        if session.is_expired() {
            self.store.remove_session(session.id).await?;
            return Err(Error::Authentication("Session expired".to_string()));
        }

        // Validate JWT token
        let validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        decode::<Claims>(
            token,
            &self.decoding_key,
            &validation,
        ).map_err(|_| Error::Authentication("Invalid token".to_string()))?;

        Ok(session)
    }

    /// Retrieves a session by ID
    pub async fn get_session(&self, session_id: Uuid) -> Result<Option<Session>> {
        self.store.get_session(session_id).await
    }

    /// Removes a session
    pub async fn remove_session(&self, session_id: Uuid) -> Result<()> {
        self.store.remove_session(session_id).await
    }

    /// Removes all sessions for a user
    pub async fn remove_user_sessions(&self, user_id: UserId) -> Result<()> {
        self.store.remove_user_sessions(user_id).await
    }

    /// Refreshes a session, extending its expiration time
    pub async fn refresh_session(&self, session_id: Uuid) -> Result<Session> {
        let session = self.store
            .get_session(session_id)
            .await?
            .ok_or_else(|| Error::Authentication("Invalid session".to_string()))?;

        // Create new session with extended expiration
        let new_session = Session::new(
            session.user_id,
            session.tenant_id,
            session.token,
            self.jwt_config.expiration,
        );

        // Store new session
        self.store.store_session(&new_session).await?;

        Ok(new_session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    // Mock session store for testing
    struct MockSessionStore {
        sessions: std::sync::Mutex<Vec<Session>>,
    }

    impl MockSessionStore {
        fn new() -> Self {
            Self {
                sessions: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl SessionStore for MockSessionStore {
        async fn store_session(&self, session: &Session) -> Result<()> {
            self.sessions.lock().unwrap().push(session.clone());
            Ok(())
        }

        async fn get_session(&self, session_id: Uuid) -> Result<Option<Session>> {
            Ok(self.sessions
                .lock()
                .unwrap()
                .iter()
                .find(|s| s.id == session_id)
                .cloned())
        }

        async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>> {
            Ok(self.sessions
                .lock()
                .unwrap()
                .iter()
                .find(|s| s.token == token)
                .cloned())
        }

        async fn remove_session(&self, session_id: Uuid) -> Result<()> {
            let mut sessions = self.sessions.lock().unwrap();
            sessions.retain(|s| s.id != session_id);
            Ok(())
        }

        async fn remove_user_sessions(&self, user_id: UserId) -> Result<()> {
            let mut sessions = self.sessions.lock().unwrap();
            sessions.retain(|s| s.user_id != user_id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let store = Box::new(MockSessionStore::new());
        let jwt_config = JwtConfig {
            secret: "test_secret".to_string(),
            issuer: "test_issuer".to_string(),
            audience: "test_audience".to_string(),
            expiration: Duration::minutes(30),
        };

        let manager = SessionManager::new(store, jwt_config);

        // Create test user
        let user = User {
            id: UserId::new(),
            tenant_id: TenantId::new(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            roles: vec![],
            active: true,
            last_login: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        };

        // Create session
        let session = manager.create_session(&user).await.unwrap();
        assert_eq!(session.user_id, user.id);
        assert_eq!(session.tenant_id, user.tenant_id);

        // Validate token
        let validated = manager.validate_token(&session.token).await.unwrap();
        assert_eq!(validated.id, session.id);

        // Refresh session
        let refreshed = manager.refresh_session(session.id).await.unwrap();
        assert_eq!(refreshed.user_id, user.id);
        assert!(refreshed.expires_at > session.expires_at);

        // Remove session
        manager.remove_session(session.id).await.unwrap();
        assert!(manager.get_session(session.id).await.unwrap().is_none());
    }
}