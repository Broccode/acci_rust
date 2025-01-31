use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    core::database::Database,
    shared::{
        error::{Error, Result},
        types::{TenantId, UserId},
    },
};

use super::models::{SsoProvider, SsoProviderType, SsoUserMapping, SsoSession};

/// Repository for SSO operations
#[derive(Debug, Clone)]
pub struct SsoRepository {
    db: Database,
}

impl SsoRepository {
    /// Creates a new SsoRepository instance
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Creates a new SSO provider
    pub async fn create_provider(&self, provider: &SsoProvider) -> Result<SsoProvider> {
        let pool = self.db.pool();
        let mut tx = pool.begin().await?;

        let result = sqlx::query!(
            r#"
            INSERT INTO sso_providers (
                id, tenant_id, name, description, provider_type, enabled,
                metadata_url, metadata_xml, entity_id, assertion_consumer_service_url,
                single_logout_url, client_id, client_secret, issuer, discovery_url,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING *
            "#,
            provider.id,
            provider.tenant_id.0,
            provider.name,
            provider.description,
            provider.provider_type.to_string(),
            provider.enabled,
            provider.metadata_url,
            provider.metadata_xml,
            provider.entity_id,
            provider.assertion_consumer_service_url,
            provider.single_logout_url,
            provider.client_id,
            provider.client_secret,
            provider.issuer,
            provider.discovery_url,
            provider.created_at,
            provider.updated_at,
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(SsoProvider {
            id: result.id,
            tenant_id: TenantId(result.tenant_id),
            name: result.name,
            description: result.description,
            provider_type: match result.provider_type.as_str() {
                "saml" => SsoProviderType::Saml,
                "oidc" => SsoProviderType::Oidc,
                _ => return Err(Error::Internal("Invalid provider type".to_string())),
            },
            enabled: result.enabled,
            metadata_url: result.metadata_url,
            metadata_xml: result.metadata_xml,
            entity_id: result.entity_id,
            assertion_consumer_service_url: result.assertion_consumer_service_url,
            single_logout_url: result.single_logout_url,
            client_id: result.client_id,
            client_secret: result.client_secret,
            issuer: result.issuer,
            discovery_url: result.discovery_url,
            created_at: result.created_at,
            updated_at: result.updated_at,
        })
    }

    /// Gets a provider by ID
    pub async fn get_provider(&self, id: Uuid) -> Result<Option<SsoProvider>> {
        let pool = self.db.pool();
        let result = sqlx::query!(
            r#"
            SELECT * FROM sso_providers WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|r| SsoProvider {
            id: r.id,
            tenant_id: TenantId(r.tenant_id),
            name: r.name,
            description: r.description,
            provider_type: match r.provider_type.as_str() {
                "saml" => SsoProviderType::Saml,
                "oidc" => SsoProviderType::Oidc,
                _ => SsoProviderType::Saml, // Default to SAML to avoid runtime errors
            },
            enabled: r.enabled,
            metadata_url: r.metadata_url,
            metadata_xml: r.metadata_xml,
            entity_id: r.entity_id,
            assertion_consumer_service_url: r.assertion_consumer_service_url,
            single_logout_url: r.single_logout_url,
            client_id: r.client_id,
            client_secret: r.client_secret,
            issuer: r.issuer,
            discovery_url: r.discovery_url,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    /// Lists all providers for a tenant
    pub async fn list_providers(&self, tenant_id: TenantId) -> Result<Vec<SsoProvider>> {
        let pool = self.db.pool();
        let results = sqlx::query!(
            r#"
            SELECT * FROM sso_providers WHERE tenant_id = $1
            "#,
            tenant_id.0,
        )
        .fetch_all(pool)
        .await?;

        Ok(results
            .into_iter()
            .map(|r| SsoProvider {
                id: r.id,
                tenant_id: TenantId(r.tenant_id),
                name: r.name,
                description: r.description,
                provider_type: match r.provider_type.as_str() {
                    "saml" => SsoProviderType::Saml,
                    "oidc" => SsoProviderType::Oidc,
                    _ => SsoProviderType::Saml,
                },
                enabled: r.enabled,
                metadata_url: r.metadata_url,
                metadata_xml: r.metadata_xml,
                entity_id: r.entity_id,
                assertion_consumer_service_url: r.assertion_consumer_service_url,
                single_logout_url: r.single_logout_url,
                client_id: r.client_id,
                client_secret: r.client_secret,
                issuer: r.issuer,
                discovery_url: r.discovery_url,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    /// Creates a new SSO user mapping
    pub async fn create_user_mapping(&self, mapping: &SsoUserMapping) -> Result<SsoUserMapping> {
        let pool = self.db.pool();
        let mut tx = pool.begin().await?;

        let result = sqlx::query!(
            r#"
            INSERT INTO sso_user_mappings (
                id, user_id, tenant_id, provider_id, external_id,
                email, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
            mapping.id,
            mapping.user_id.0,
            mapping.tenant_id.0,
            mapping.provider_id,
            mapping.external_id,
            mapping.email,
            mapping.created_at,
            mapping.updated_at,
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(SsoUserMapping {
            id: result.id,
            user_id: UserId(result.user_id),
            tenant_id: TenantId(result.tenant_id),
            provider_id: result.provider_id,
            external_id: result.external_id,
            email: result.email,
            created_at: result.created_at,
            updated_at: result.updated_at,
        })
    }

    /// Gets a user mapping by external ID
    pub async fn get_user_mapping(
        &self,
        provider_id: Uuid,
        external_id: &str,
    ) -> Result<Option<SsoUserMapping>> {
        let pool = self.db.pool();
        let result = sqlx::query!(
            r#"
            SELECT * FROM sso_user_mappings
            WHERE provider_id = $1 AND external_id = $2
            "#,
            provider_id,
            external_id,
        )
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|r| SsoUserMapping {
            id: r.id,
            user_id: UserId(r.user_id),
            tenant_id: TenantId(r.tenant_id),
            provider_id: r.provider_id,
            external_id: r.external_id,
            email: r.email,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    /// Creates a new SSO session
    pub async fn create_session(&self, session: &SsoSession) -> Result<SsoSession> {
        let pool = self.db.pool();
        let mut tx = pool.begin().await?;

        let result = sqlx::query!(
            r#"
            INSERT INTO sso_sessions (
                id, user_id, tenant_id, provider_id, session_index,
                name_id, created_at, expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
            session.id,
            session.user_id.0,
            session.tenant_id.0,
            session.provider_id,
            session.session_index,
            session.name_id,
            session.created_at,
            session.expires_at,
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(SsoSession {
            id: result.id,
            user_id: UserId(result.user_id),
            tenant_id: TenantId(result.tenant_id),
            provider_id: result.provider_id,
            session_index: result.session_index,
            name_id: result.name_id,
            created_at: result.created_at,
            expires_at: result.expires_at,
        })
    }

    /// Gets a session by ID
    pub async fn get_session(&self, id: Uuid) -> Result<Option<SsoSession>> {
        let pool = self.db.pool();
        let result = sqlx::query!(
            r#"
            SELECT * FROM sso_sessions WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|r| SsoSession {
            id: r.id,
            user_id: UserId(r.user_id),
            tenant_id: TenantId(r.tenant_id),
            provider_id: r.provider_id,
            session_index: r.session_index,
            name_id: r.name_id,
            created_at: r.created_at,
            expires_at: r.expires_at,
        }))
    }

    /// Deletes expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<u64> {
        let pool = self.db.pool();
        let result = sqlx::query!(
            r#"
            DELETE FROM sso_sessions
            WHERE expires_at <= NOW()
            "#,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[tokio::test]
    async fn test_sso_provider_crud() {
        let config = crate::core::config::DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "acci_rust_test".to_string(),
            max_connections: 5,
            ssl_mode: false,
        };

        let db = Database::connect(&config).await.unwrap();
        let repository = SsoRepository::new(db);

        // Create tenant first
        let tenant_id = TenantId::new();
        sqlx::query!(
            r#"
            INSERT INTO tenants (id, name)
            VALUES ($1, $2)
            "#,
            tenant_id.0,
            "Test Tenant",
        )
        .execute(repository.db.pool())
        .await
        .unwrap();

        // Test SAML provider
        let provider = SsoProvider::new_saml(
            tenant_id,
            "Test SAML".to_string(),
            Some("Test Provider".to_string()),
            Some("https://metadata.url".to_string()),
            None,
            "entity_id".to_string(),
            "https://acs.url".to_string(),
            Some("https://slo.url".to_string()),
        );

        let created = repository.create_provider(&provider).await.unwrap();
        assert_eq!(created.name, provider.name);

        let retrieved = repository.get_provider(created.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, created.id);

        let providers = repository.list_providers(tenant_id).await.unwrap();
        assert!(!providers.is_empty());
        assert!(providers.iter().any(|p| p.id == created.id));
    }

    #[tokio::test]
    async fn test_sso_user_mapping() {
        let config = crate::core::config::DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "acci_rust_test".to_string(),
            max_connections: 5,
            ssl_mode: false,
        };

        let db = Database::connect(&config).await.unwrap();
        let repository = SsoRepository::new(db);

        // Create tenant and user first
        let tenant_id = TenantId::new();
        let user_id = UserId::new();

        sqlx::query!(
            r#"
            INSERT INTO tenants (id, name)
            VALUES ($1, $2)
            "#,
            tenant_id.0,
            "Test Tenant",
        )
        .execute(repository.db.pool())
        .await
        .unwrap();

        sqlx::query!(
            r#"
            INSERT INTO users (id, tenant_id, email, password_hash)
            VALUES ($1, $2, $3, $4)
            "#,
            user_id.0,
            tenant_id.0,
            "test@example.com",
            "hash",
        )
        .execute(repository.db.pool())
        .await
        .unwrap();

        // Create provider
        let provider = SsoProvider::new_saml(
            tenant_id,
            "Test SAML".to_string(),
            None,
            None,
            None,
            "entity_id".to_string(),
            "https://acs.url".to_string(),
            None,
        );

        let provider = repository.create_provider(&provider).await.unwrap();

        // Test user mapping
        let mapping = SsoUserMapping::new(
            user_id,
            tenant_id,
            provider.id,
            "external_id".to_string(),
            "test@example.com".to_string(),
        );

        let created = repository.create_user_mapping(&mapping).await.unwrap();
        assert_eq!(created.external_id, mapping.external_id);

        let retrieved = repository
            .get_user_mapping(provider.id, &mapping.external_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.id, created.id);
    }

    #[tokio::test]
    async fn test_sso_session() {
        let config = crate::core::config::DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "acci_rust_test".to_string(),
            max_connections: 5,
            ssl_mode: false,
        };

        let db = Database::connect(&config).await.unwrap();
        let repository = SsoRepository::new(db);

        // Create tenant and user first
        let tenant_id = TenantId::new();
        let user_id = UserId::new();

        sqlx::query!(
            r#"
            INSERT INTO tenants (id, name)
            VALUES ($1, $2)
            "#,
            tenant_id.0,
            "Test Tenant",
        )
        .execute(repository.db.pool())
        .await
        .unwrap();

        sqlx::query!(
            r#"
            INSERT INTO users (id, tenant_id, email, password_hash)
            VALUES ($1, $2, $3, $4)
            "#,
            user_id.0,
            tenant_id.0,
            "test@example.com",
            "hash",
        )
        .execute(repository.db.pool())
        .await
        .unwrap();

        // Create provider
        let provider = SsoProvider::new_saml(
            tenant_id,
            "Test SAML".to_string(),
            None,
            None,
            None,
            "entity_id".to_string(),
            "https://acs.url".to_string(),
            None,
        );

        let provider = repository.create_provider(&provider).await.unwrap();

        // Test session
        let session = SsoSession::new(
            user_id,
            tenant_id,
            provider.id,
            Some("session_index".to_string()),
            Some("name_id".to_string()),
            OffsetDateTime::now_utc() + Duration::hours(1),
        );

        let created = repository.create_session(&session).await.unwrap();
        assert_eq!(created.session_index, session.session_index);

        let retrieved = repository.get_session(created.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, created.id);

        // Test cleanup
        let expired_session = SsoSession::new(
            user_id,
            tenant_id,
            provider.id,
            None,
            None,
            OffsetDateTime::now_utc() - Duration::minutes(1),
        );

        repository.create_session(&expired_session).await.unwrap();

        let cleaned = repository.cleanup_expired_sessions().await.unwrap();
        assert_eq!(cleaned, 1);
    }
}