use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
};

use super::{
    models::{SsoProvider, SsoProviderType, SsoSession, SsoUserMapping},
    oidc::{OidcConfig, OidcService},
    repository::SsoRepository,
    saml::{SamlConfig, SamlService},
};

/// SSO service configuration
#[derive(Debug, Clone)]
pub struct SsoConfig {
    pub saml: SamlConfig,
    pub oidc: OidcConfig,
}

/// SSO service for handling authentication
#[derive(Debug)]
pub struct SsoService {
    repository: SsoRepository,
    saml_service: SamlService,
    oidc_service: OidcService,
}

impl SsoService {
    /// Creates a new SsoService instance
    pub fn new(repository: SsoRepository) -> Self {
        let saml_config = SamlConfig {
            certificate: std::env::var("SAML_CERTIFICATE")
                .expect("SAML_CERTIFICATE must be set"),
            private_key: std::env::var("SAML_PRIVATE_KEY")
                .expect("SAML_PRIVATE_KEY must be set"),
            organization_name: std::env::var("SAML_ORG_NAME")
                .expect("SAML_ORG_NAME must be set"),
            organization_display_name: std::env::var("SAML_ORG_DISPLAY_NAME")
                .expect("SAML_ORG_DISPLAY_NAME must be set"),
            organization_url: std::env::var("SAML_ORG_URL")
                .expect("SAML_ORG_URL must be set"),
            technical_contact_name: std::env::var("SAML_TECH_CONTACT_NAME")
                .expect("SAML_TECH_CONTACT_NAME must be set"),
            technical_contact_email: std::env::var("SAML_TECH_CONTACT_EMAIL")
                .expect("SAML_TECH_CONTACT_EMAIL must be set"),
        };

        let oidc_config = OidcConfig {
            redirect_url: std::env::var("OIDC_REDIRECT_URL")
                .expect("OIDC_REDIRECT_URL must be set"),
        };

        Self {
            repository,
            saml_service: SamlService::new(saml_config),
            oidc_service: OidcService::new(oidc_config),
        }
    }

    /// Creates a new SSO provider
    pub async fn create_provider(&self, provider: &SsoProvider) -> Result<SsoProvider> {
        // Validate provider configuration
        match provider.provider_type {
            SsoProviderType::Saml => {
                if provider.entity_id.is_none() || provider.assertion_consumer_service_url.is_none() {
                    return Err(Error::InvalidInput(
                        "SAML provider requires entity_id and assertion_consumer_service_url"
                            .to_string(),
                    ));
                }
            }
            SsoProviderType::Oidc => {
                if provider.client_id.is_none()
                    || provider.client_secret.is_none()
                    || provider.issuer.is_none()
                {
                    return Err(Error::InvalidInput(
                        "OIDC provider requires client_id, client_secret, and issuer".to_string(),
                    ));
                }
            }
        }

        self.repository.create_provider(provider).await
    }

    /// Gets a provider by ID
    pub async fn get_provider(&self, id: Uuid) -> Result<Option<SsoProvider>> {
        self.repository.get_provider(id).await
    }

    /// Lists all providers for a tenant
    pub async fn list_providers(&self, tenant_id: TenantId) -> Result<Vec<SsoProvider>> {
        self.repository.list_providers(tenant_id).await
    }

    /// Initiates SSO authentication
    pub async fn initiate_auth(
        &self,
        provider: &SsoProvider,
    ) -> Result<(String, Option<String>, Option<String>)> {
        if !provider.enabled {
            return Err(Error::Authentication(
                "SSO provider is disabled".to_string(),
            ));
        }

        match provider.provider_type {
            SsoProviderType::Saml => {
                let (request, relay_state) = self.saml_service.create_auth_request(provider)?;
                Ok((request, Some(relay_state), None))
            }
            SsoProviderType::Oidc => {
                let (url, csrf_token, nonce) = self.oidc_service.create_auth_url(provider).await?;
                Ok((
                    url.to_string(),
                    Some(csrf_token.secret().to_string()),
                    Some(nonce.secret().to_string()),
                ))
            }
        }
    }

    /// Validates SSO response
    pub async fn validate_response(
        &self,
        provider: &SsoProvider,
        response: &str,
        relay_state: Option<&str>,
        nonce: Option<&str>,
    ) -> Result<(String, String)> {
        if !provider.enabled {
            return Err(Error::Authentication(
                "SSO provider is disabled".to_string(),
            ));
        }

        match provider.provider_type {
            SsoProviderType::Saml => {
                let relay_state = relay_state.ok_or_else(|| {
                    Error::Authentication("Missing SAML relay state".to_string())
                })?;

                let (name_id, session_index, email) =
                    self.saml_service
                        .validate_response(provider, response, relay_state)?;

                // Create SSO session if session index is provided
                if let Some(session_index) = session_index {
                    self.create_session(
                        provider.id,
                        &name_id,
                        Some(session_index),
                        Some(name_id.clone()),
                    )
                    .await?;
                }

                Ok((name_id, email.unwrap_or_else(|| name_id.clone())))
            }
            SsoProviderType::Oidc => {
                let nonce = nonce.ok_or_else(|| {
                    Error::Authentication("Missing OIDC nonce".to_string())
                })?;

                let (subject, email) = self
                    .oidc_service
                    .validate_auth_code(
                        provider,
                        response,
                        openidconnect::Nonce::new(nonce.to_string()),
                    )
                    .await?;

                Ok((subject, email))
            }
        }
    }

    /// Creates a user mapping
    pub async fn create_user_mapping(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        provider_id: Uuid,
        external_id: String,
        email: String,
    ) -> Result<SsoUserMapping> {
        let mapping = SsoUserMapping::new(user_id, tenant_id, provider_id, external_id, email);
        self.repository.create_user_mapping(&mapping).await
    }

    /// Gets a user mapping by external ID
    pub async fn get_user_mapping(
        &self,
        provider_id: Uuid,
        external_id: &str,
    ) -> Result<Option<SsoUserMapping>> {
        self.repository
            .get_user_mapping(provider_id, external_id)
            .await
    }

    /// Creates an SSO session
    pub async fn create_session(
        &self,
        provider_id: Uuid,
        user_id: &str,
        session_index: Option<String>,
        name_id: Option<String>,
    ) -> Result<SsoSession> {
        // Get user mapping
        let mapping = self
            .get_user_mapping(provider_id, user_id)
            .await?
            .ok_or_else(|| Error::Authentication("User mapping not found".to_string()))?;

        let session = SsoSession::new(
            mapping.user_id,
            mapping.tenant_id,
            provider_id,
            session_index,
            name_id,
            OffsetDateTime::now_utc() + Duration::hours(8),
        );

        self.repository.create_session(&session).await
    }

    /// Gets a session by ID
    pub async fn get_session(&self, id: Uuid) -> Result<Option<SsoSession>> {
        self.repository.get_session(id).await
    }

    /// Cleans up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<u64> {
        self.repository.cleanup_expired_sessions().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{config::DatabaseConfig, database::Database};

    async fn create_test_service() -> SsoService {
        let config = DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "acci_rust_test".to_string(),
            max_connections: 5,
            ssl_mode: false,
        };

        // Set required environment variables for testing
        std::env::set_var("SAML_CERTIFICATE", "test_cert");
        std::env::set_var("SAML_PRIVATE_KEY", "test_key");
        std::env::set_var("SAML_ORG_NAME", "Test Org");
        std::env::set_var("SAML_ORG_DISPLAY_NAME", "Test Organization");
        std::env::set_var("SAML_ORG_URL", "https://test.org");
        std::env::set_var("SAML_TECH_CONTACT_NAME", "Test Admin");
        std::env::set_var("SAML_TECH_CONTACT_EMAIL", "admin@test.org");
        std::env::set_var(
            "OIDC_REDIRECT_URL",
            "http://localhost:3000/auth/callback",
        );

        let db = Database::connect(&config).await.unwrap();
        let repository = SsoRepository::new(db);
        SsoService::new(repository)
    }

    #[tokio::test]
    async fn test_sso_provider_management() {
        let service = create_test_service().await;

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
        .execute(service.repository.db.pool())
        .await
        .unwrap();

        // Test SAML provider
        let provider = SsoProvider::new_saml(
            tenant_id,
            "Test SAML".to_string(),
            Some("Test Provider".to_string()),
            Some("https://metadata.url".to_string()),
            None,
            "https://test.org/sp".to_string(),
            "https://test.org/acs".to_string(),
            Some("https://test.org/slo".to_string()),
        );

        let created = service.create_provider(&provider).await.unwrap();
        assert_eq!(created.name, provider.name);

        let providers = service.list_providers(tenant_id).await.unwrap();
        assert!(!providers.is_empty());
        assert!(providers.iter().any(|p| p.id == created.id));
    }

    #[tokio::test]
    async fn test_sso_user_mapping() {
        let service = create_test_service().await;

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
        .execute(service.repository.db.pool())
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
        .execute(service.repository.db.pool())
        .await
        .unwrap();

        // Create provider
        let provider = SsoProvider::new_saml(
            tenant_id,
            "Test SAML".to_string(),
            None,
            None,
            None,
            "https://test.org/sp".to_string(),
            "https://test.org/acs".to_string(),
            None,
        );

        let provider = service.create_provider(&provider).await.unwrap();

        // Test user mapping
        let mapping = service
            .create_user_mapping(
                user_id,
                tenant_id,
                provider.id,
                "external_id".to_string(),
                "test@example.com".to_string(),
            )
            .await
            .unwrap();

        let retrieved = service
            .get_user_mapping(provider.id, &mapping.external_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.id, mapping.id);
    }
}