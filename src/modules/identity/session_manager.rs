use jsonwebtoken::{DecodingKey, EncodingKey};
use uuid::Uuid;

use crate::{
    modules::identity::session::{Claims, JwtConfig, RedisSessionStore, Session, SessionStore},
    shared::{
        error::{Error, Result},
        types::{TenantId, UserId},
    },
};

/// Session manager for handling user sessions
pub struct SessionManager {
    store: RedisSessionStore,
    jwt_config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl SessionManager {
    /// Creates a new SessionManager instance
    pub fn new(store: RedisSessionStore, jwt_config: JwtConfig) -> Self {
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
    pub async fn create_session(&self, user_id: UserId, tenant_id: TenantId) -> Result<Session> {
        let claims = Claims::new(
            user_id,
            tenant_id,
            self.jwt_config.issuer.clone(),
            self.jwt_config.audience.clone(),
            self.jwt_config.expiration,
        );

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &self.encoding_key,
        )
        .map_err(|e| Error::Internal(format!("Failed to create JWT: {}", e)))?;

        let session = Session::new(user_id, tenant_id, token, self.jwt_config.expiration);
        self.store.store_session(&session).await?;
        Ok(session)
    }

    /// Validates a session token
    pub async fn validate_token(&self, token: &str) -> Result<Session> {
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.set_audience(&[&self.jwt_config.audience]);
        validation.set_issuer(&[&self.jwt_config.issuer]);

        let claims: Claims = jsonwebtoken::decode(token, &self.decoding_key, &validation)
            .map_err(|e| Error::Authentication(format!("Invalid session token: {}", e)))?
            .claims;

        let session = self
            .store
            .get_session_by_token(token)
            .await?
            .ok_or_else(|| Error::Authentication("Session not found".to_string()))?;

        if session.is_expired() {
            return Err(Error::Authentication("Session expired".to_string()));
        }

        Ok(session)
    }

    /// Gets a session by ID
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

    /// Refreshes a session
    pub async fn refresh_session(&self, session_id: Uuid) -> Result<Session> {
        let session = self
            .store
            .get_session(session_id)
            .await?
            .ok_or_else(|| Error::Authentication("Session not found".to_string()))?;

        let claims = Claims::new(
            session.user_id,
            session.tenant_id,
            self.jwt_config.issuer.clone(),
            self.jwt_config.audience.clone(),
            self.jwt_config.expiration,
        );

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &self.encoding_key,
        )
        .map_err(|e| Error::Internal(format!("Failed to create JWT: {}", e)))?;

        let new_session = Session::new(
            session.user_id,
            session.tenant_id,
            token,
            self.jwt_config.expiration,
        );

        self.store.store_session(&new_session).await?;
        self.store.remove_session(session_id).await?;

        Ok(new_session)
    }

    pub async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>> {
        self.store.get_session_by_token(token).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Arc;
    use testcontainers::*;
    use testcontainers_modules::redis::Redis;

    static DOCKER: Lazy<Arc<clients::Cli>> = Lazy::new(|| Arc::new(clients::Cli::default()));

    async fn create_test_session_manager() -> (SessionManager, Container<'static, Redis>) {
        let redis_container = DOCKER.run(Redis::default());
        let port = redis_container.get_host_port_ipv4(6379);
        let redis_url = format!("redis://127.0.0.1:{}", port);

        let store = RedisSessionStore::new(&redis_url).expect("Failed to create Redis store");
        let jwt_config = JwtConfig {
            secret: "test_secret".to_string(),
            issuer: "test_issuer".to_string(),
            audience: "test_audience".to_string(),
            expiration: Duration::hours(1),
        };
        let manager = SessionManager::new(store, jwt_config);
        (manager, redis_container)
    }

    #[tokio::test]
    async fn test_session_management() {
        let (manager, _container) = create_test_session_manager().await;
        let user_id = UserId::new();
        let tenant_id = TenantId::new();

        // Create session
        let session = manager.create_session(user_id, tenant_id).await.unwrap();

        // Validate token
        let validated = manager.validate_token(&session.token).await.unwrap();
        assert_eq!(validated.id, session.id);
        assert_eq!(validated.user_id, user_id);
        assert_eq!(validated.tenant_id, tenant_id);

        // Get session
        let retrieved = manager.get_session(session.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, session.id);
        assert_eq!(retrieved.user_id, user_id);
        assert_eq!(retrieved.tenant_id, tenant_id);

        // Get by token
        let retrieved = manager
            .get_session_by_token(&session.token)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.id, session.id);

        // Remove session
        manager.remove_session(session.id).await.unwrap();
        assert!(manager.get_session(session.id).await.unwrap().is_none());

        // Test user sessions
        let session2 = manager.create_session(user_id, tenant_id).await.unwrap();

        // Remove all user sessions
        manager.remove_user_sessions(user_id).await.unwrap();
        assert!(manager.get_session(session2.id).await.unwrap().is_none());
    }
}
