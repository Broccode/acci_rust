use redis::{aio::Connection, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
};

/// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub audience: String,
    pub expiration: Duration,
}

/// JWT claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub iss: String,
    pub aud: String,
    pub tenant_id: String,
}

impl Claims {
    /// Creates new JWT claims
    pub fn new(
        user_id: UserId,
        tenant_id: TenantId,
        issuer: String,
        audience: String,
        expiration: Duration,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            sub: user_id.0.to_string(),
            exp: (now + expiration).unix_timestamp(),
            iat: now.unix_timestamp(),
            iss: issuer,
            aud: audience,
            tenant_id: tenant_id.0.to_string(),
        }
    }
}

/// Session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub token: String,
    pub expires_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
}

impl Session {
    /// Creates a new session
    pub fn new(user_id: UserId, tenant_id: TenantId, token: String, expires_in: Duration) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: Uuid::new_v4(),
            user_id,
            tenant_id,
            token,
            expires_at: now + expires_in,
            created_at: now,
        }
    }

    /// Checks if the session is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at <= OffsetDateTime::now_utc()
    }
}

/// Session store trait
#[async_trait::async_trait]
pub trait SessionStore: Send + Sync + std::fmt::Debug + 'static {
    /// Stores a session
    async fn store_session(&self, session: &Session) -> Result<()>;

    /// Gets a session by ID
    async fn get_session(&self, session_id: Uuid) -> Result<Option<Session>>;

    /// Gets a session by token
    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>>;

    /// Removes a session
    async fn remove_session(&self, session_id: Uuid) -> Result<()>;

    /// Removes all sessions for a user
    async fn remove_user_sessions(&self, user_id: UserId) -> Result<()>;
}

/// Redis session store
#[derive(Debug)]
pub struct RedisSessionStore {
    client: Client,
}

impl RedisSessionStore {
    /// Creates a new RedisSessionStore
    pub fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| Error::Database(format!("Failed to connect to Redis: {}", e)))?;
        Ok(Self { client })
    }

    /// Gets a Redis connection
    async fn get_connection(&self) -> Result<Connection> {
        self.client
            .get_async_connection()
            .await
            .map_err(|e| Error::Database(format!("Failed to get Redis connection: {}", e)))
    }
}

#[async_trait::async_trait]
impl SessionStore for RedisSessionStore {
    async fn store_session(&self, session: &Session) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let key = format!("session:{}", session.id);
        let token_key = format!("token:{}", session.token);
        let user_key = format!("user:{}:sessions", session.user_id.0);

        // Store session data
        let session_data = serde_json::to_string(session)
            .map_err(|e| Error::Internal(format!("Failed to serialize session: {}", e)))?;

        // Set session data with expiration
        let ttl = (session.expires_at - OffsetDateTime::now_utc()).whole_seconds();
        redis::pipe()
            .atomic()
            .set(&key, &session_data)
            .expire(&key, ttl)
            .set(&token_key, session.id.to_string())
            .expire(&token_key, ttl)
            .sadd(&user_key, session.id.to_string())
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Database(format!("Failed to store session: {}", e)))?;

        Ok(())
    }

    async fn get_session(&self, session_id: Uuid) -> Result<Option<Session>> {
        let mut conn = self.get_connection().await?;
        let key = format!("session:{}", session_id);

        let data: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| Error::Database(format!("Failed to get session: {}", e)))?;

        match data {
            Some(data) => {
                let session: Session = serde_json::from_str(&data).map_err(|e| {
                    Error::Internal(format!("Failed to deserialize session: {}", e))
                })?;
                Ok(Some(session))
            },
            None => Ok(None),
        }
    }

    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>> {
        let mut conn = self.get_connection().await?;
        let token_key = format!("token:{}", token);

        let session_id: Option<String> = conn
            .get(&token_key)
            .await
            .map_err(|e| Error::Database(format!("Failed to get session ID: {}", e)))?;

        match session_id {
            Some(id) => {
                let session_id = Uuid::parse_str(&id)
                    .map_err(|e| Error::Internal(format!("Invalid session ID: {}", e)))?;
                self.get_session(session_id).await
            },
            None => Ok(None),
        }
    }

    async fn remove_session(&self, session_id: Uuid) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let key = format!("session:{}", session_id);

        // Get session data to remove token and user references
        if let Some(session) = self.get_session(session_id).await? {
            let token_key = format!("token:{}", session.token);
            let user_key = format!("user:{}:sessions", session.user_id.0);

            redis::pipe()
                .atomic()
                .del(&key)
                .del(&token_key)
                .srem(&user_key, session_id.to_string())
                .query_async(&mut conn)
                .await
                .map_err(|e| Error::Database(format!("Failed to remove session: {}", e)))?;
        }

        Ok(())
    }

    async fn remove_user_sessions(&self, user_id: UserId) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let user_key = format!("user:{}:sessions", user_id.0);

        // Get all session IDs for user
        let session_ids: Vec<String> = conn
            .smembers(&user_key)
            .await
            .map_err(|e| Error::Database(format!("Failed to get user sessions: {}", e)))?;

        // Remove each session
        for id in session_ids {
            let session_id = Uuid::parse_str(&id)
                .map_err(|e| Error::Internal(format!("Invalid session ID: {}", e)))?;
            self.remove_session(session_id).await?;
        }

        Ok(())
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

    async fn create_redis_store() -> (RedisSessionStore, Container<'static, Redis>) {
        let redis_container = DOCKER.run(Redis::default());
        let port = redis_container.get_host_port_ipv4(6379);
        let redis_url = format!("redis://127.0.0.1:{}", port);

        let store = RedisSessionStore::new(&redis_url).expect("Failed to create Redis store");
        (store, redis_container)
    }

    #[tokio::test]
    async fn test_session_store() {
        let (store, _container) = create_redis_store().await;
        let session = Session::new(
            UserId::new(),
            TenantId::new(),
            "test_token".to_string(),
            Duration::hours(1),
        );

        // Test storing session
        store.store_session(&session).await.unwrap();

        // Test retrieving session by ID
        let retrieved = store.get_session(session.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, session.id);
        assert_eq!(retrieved.user_id, session.user_id);
        assert_eq!(retrieved.token, session.token);

        // Test retrieving session by token
        let retrieved = store
            .get_session_by_token(&session.token)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.id, session.id);
        assert_eq!(retrieved.user_id, session.user_id);
        assert_eq!(retrieved.token, session.token);

        // Test removing session
        store.remove_session(session.id).await.unwrap();
        assert!(store.get_session(session.id).await.unwrap().is_none());

        // Test user sessions
        let session2 = Session::new(
            session.user_id,
            TenantId::new(),
            "test_token_2".to_string(),
            Duration::hours(1),
        );
        store.store_session(&session2).await.unwrap();

        // Remove all user sessions
        store.remove_user_sessions(session.user_id).await.unwrap();
        assert!(store.get_session(session2.id).await.unwrap().is_none());
    }

    #[test]
    fn test_claims_creation() {
        let user_id = UserId::new();
        let tenant_id = TenantId::new();
        let issuer = "test_issuer".to_string();
        let audience = "test_audience".to_string();
        let expiration = Duration::hours(1);

        let claims = Claims::new(
            user_id,
            tenant_id,
            issuer.clone(),
            audience.clone(),
            expiration,
        );

        assert_eq!(claims.sub, user_id.0.to_string());
        assert_eq!(claims.tenant_id, tenant_id.0.to_string());
        assert_eq!(claims.iss, issuer);
        assert_eq!(claims.aud, audience);
        assert!(claims.exp > claims.iat);
    }
}
