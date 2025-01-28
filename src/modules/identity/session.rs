use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
};

/// Represents a user session
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
    pub fn new(user_id: UserId, tenant_id: TenantId, token: String, expires_in: time::Duration) -> Self {
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
        OffsetDateTime::now_utc() > self.expires_at
    }
}

/// Configuration for JWT tokens
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub audience: String,
    pub expiration: time::Duration,
}

/// Claims for JWT tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Issuer
    pub iss: String,
    /// Subject (User ID)
    pub sub: String,
    /// Audience
    pub aud: String,
    /// Expiration time (as UTC timestamp)
    pub exp: i64,
    /// Issued at (as UTC timestamp)
    pub iat: i64,
    /// JWT ID
    pub jti: String,
    /// Tenant ID
    pub tid: String,
}

impl Claims {
    /// Creates new claims for a user
    pub fn new(
        user_id: UserId,
        tenant_id: TenantId,
        config: &JwtConfig,
        session_id: Uuid,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            iss: config.issuer.clone(),
            sub: user_id.0.to_string(),
            aud: config.audience.clone(),
            exp: (now + config.expiration).unix_timestamp(),
            iat: now.unix_timestamp(),
            jti: session_id.to_string(),
            tid: tenant_id.0.to_string(),
        }
    }
}

/// Session store for managing user sessions
#[async_trait::async_trait]
pub trait SessionStore: Send + Sync + 'static {
    /// Stores a session
    async fn store_session(&self, session: &Session) -> Result<()>;

    /// Retrieves a session by ID
    async fn get_session(&self, session_id: Uuid) -> Result<Option<Session>>;

    /// Retrieves a session by token
    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>>;

    /// Removes a session
    async fn remove_session(&self, session_id: Uuid) -> Result<()>;

    /// Removes all sessions for a user
    async fn remove_user_sessions(&self, user_id: UserId) -> Result<()>;
}

/// Redis implementation of SessionStore
pub struct RedisSessionStore {
    client: redis::Client,
}

impl RedisSessionStore {
    /// Creates a new RedisSessionStore
    pub fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| Error::Internal(format!("Failed to connect to Redis: {}", e)))?;
        Ok(Self { client })
    }

    /// Gets a Redis connection from the pool
    async fn get_connection(&self) -> Result<redis::aio::Connection> {
        self.client
            .get_async_connection()
            .await
            .map_err(|e| Error::Internal(format!("Failed to get Redis connection: {}", e)))
    }

    /// Generates a session key for Redis
    fn session_key(session_id: Uuid) -> String {
        format!("session:{}", session_id)
    }

    /// Generates a token key for Redis
    fn token_key(token: &str) -> String {
        format!("token:{}", token)
    }

    /// Generates a user sessions key for Redis
    fn user_sessions_key(user_id: UserId) -> String {
        format!("user:{}:sessions", user_id.0)
    }
}

#[async_trait::async_trait]
impl SessionStore for RedisSessionStore {
    async fn store_session(&self, session: &Session) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let session_key = Self::session_key(session.id);
        let token_key = Self::token_key(&session.token);
        let user_sessions_key = Self::user_sessions_key(session.user_id);

        // Store session data
        let session_json = serde_json::to_string(session)
            .map_err(|e| Error::Internal(format!("Failed to serialize session: {}", e)))?;

        let expiry_seconds = (session.expires_at - OffsetDateTime::now_utc())
            .whole_seconds()
            .max(0) as u64;

        // Use a Redis transaction to ensure atomicity
        redis::pipe()
            .atomic()
            // Store session data with expiration
            .set_ex(
                &session_key,
                &session_json,
                expiry_seconds,
            )
            // Store token to session ID mapping
            .set_ex(
                &token_key,
                session.id.to_string(),
                expiry_seconds,
            )
            // Add session ID to user's sessions set
            .sadd(&user_sessions_key, session.id.to_string())
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Internal(format!("Failed to store session: {}", e)))?;

        Ok(())
    }

    async fn get_session(&self, session_id: Uuid) -> Result<Option<Session>> {
        let mut conn = self.get_connection().await?;
        let session_key = Self::session_key(session_id);

        let session_data: Option<String> = redis::cmd("GET")
            .arg(&session_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Internal(format!("Failed to get session: {}", e)))?;

        match session_data {
            Some(data) => {
                let session: Session = serde_json::from_str(&data)
                    .map_err(|e| Error::Internal(format!("Failed to deserialize session: {}", e)))?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>> {
        let mut conn = self.get_connection().await?;
        let token_key = Self::token_key(token);

        let session_id: Option<String> = redis::cmd("GET")
            .arg(&token_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Internal(format!("Failed to get session ID: {}", e)))?;

        match session_id {
            Some(id) => {
                let session_id = Uuid::parse_str(&id)
                    .map_err(|e| Error::Internal(format!("Invalid session ID: {}", e)))?;
                self.get_session(session_id).await
            }
            None => Ok(None),
        }
    }

    async fn remove_session(&self, session_id: Uuid) -> Result<()> {
        let mut conn = self.get_connection().await?;
        
        // Get session first to get the token
        if let Some(session) = self.get_session(session_id).await? {
            let session_key = Self::session_key(session_id);
            let token_key = Self::token_key(&session.token);
            let user_sessions_key = Self::user_sessions_key(session.user_id);

            redis::pipe()
                .atomic()
                .del(&session_key)
                .del(&token_key)
                .srem(&user_sessions_key, session_id.to_string())
                .query_async(&mut conn)
                .await
                .map_err(|e| Error::Internal(format!("Failed to remove session: {}", e)))?;
        }

        Ok(())
    }

    async fn remove_user_sessions(&self, user_id: UserId) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let user_sessions_key = Self::user_sessions_key(user_id);

        // Get all session IDs for the user
        let session_ids: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&user_sessions_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Internal(format!("Failed to get user sessions: {}", e)))?;

        // Remove each session
        for id_str in session_ids {
            if let Ok(session_id) = Uuid::parse_str(&id_str) {
                self.remove_session(session_id).await?;
            }
        }

        // Remove the user's sessions set
        redis::cmd("DEL")
            .arg(&user_sessions_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Internal(format!("Failed to remove user sessions set: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[test]
    fn test_session_expiration() {
        let user_id = UserId::new();
        let tenant_id = TenantId::new();
        let token = "test_token".to_string();
        let expires_in = Duration::minutes(30);

        let session = Session::new(user_id, tenant_id, token, expires_in);
        assert!(!session.is_expired());
    }

    #[test]
    fn test_claims_creation() {
        let user_id = UserId::new();
        let tenant_id = TenantId::new();
        let session_id = Uuid::new_v4();

        let config = JwtConfig {
            secret: "test_secret".to_string(),
            issuer: "test_issuer".to_string(),
            audience: "test_audience".to_string(),
            expiration: Duration::minutes(30),
        };

        let claims = Claims::new(user_id, tenant_id, &config, session_id);
        assert_eq!(claims.iss, config.issuer);
        assert_eq!(claims.sub, user_id.0.to_string());
        assert_eq!(claims.aud, config.audience);
        assert_eq!(claims.tid, tenant_id.0.to_string());
        assert_eq!(claims.jti, session_id.to_string());
    }
}