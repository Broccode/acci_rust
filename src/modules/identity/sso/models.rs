use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::shared::types::{TenantId, UserId};

/// SSO provider type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SsoProviderType {
    /// SAML 2.0 provider
    Saml,
    /// OpenID Connect provider
    Oidc,
}

impl std::fmt::Display for SsoProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SsoProviderType::Saml => write!(f, "saml"),
            SsoProviderType::Oidc => write!(f, "oidc"),
        }
    }
}

/// SSO provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProvider {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub description: Option<String>,
    pub provider_type: SsoProviderType,
    pub enabled: bool,
    pub metadata_url: Option<String>,
    pub metadata_xml: Option<String>,
    pub entity_id: Option<String>,
    pub assertion_consumer_service_url: Option<String>,
    pub single_logout_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub issuer: Option<String>,
    pub discovery_url: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl SsoProvider {
    /// Creates a new SAML provider
    pub fn new_saml(
        tenant_id: TenantId,
        name: String,
        description: Option<String>,
        metadata_url: Option<String>,
        metadata_xml: Option<String>,
        entity_id: String,
        assertion_consumer_service_url: String,
        single_logout_url: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id,
            name,
            description,
            provider_type: SsoProviderType::Saml,
            enabled: true,
            metadata_url,
            metadata_xml,
            entity_id: Some(entity_id),
            assertion_consumer_service_url: Some(assertion_consumer_service_url),
            single_logout_url,
            client_id: None,
            client_secret: None,
            issuer: None,
            discovery_url: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    /// Creates a new OIDC provider
    pub fn new_oidc(
        tenant_id: TenantId,
        name: String,
        description: Option<String>,
        client_id: String,
        client_secret: String,
        issuer: String,
        discovery_url: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id,
            name,
            description,
            provider_type: SsoProviderType::Oidc,
            enabled: true,
            metadata_url: None,
            metadata_xml: None,
            entity_id: None,
            assertion_consumer_service_url: None,
            single_logout_url: None,
            client_id: Some(client_id),
            client_secret: Some(client_secret),
            issuer: Some(issuer),
            discovery_url,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }
}

/// SSO user mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoUserMapping {
    pub id: Uuid,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub provider_id: Uuid,
    pub external_id: String,
    pub email: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl SsoUserMapping {
    /// Creates a new SSO user mapping
    pub fn new(
        user_id: UserId,
        tenant_id: TenantId,
        provider_id: Uuid,
        external_id: String,
        email: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            tenant_id,
            provider_id,
            external_id,
            email,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }
}

/// SSO session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoSession {
    pub id: Uuid,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub provider_id: Uuid,
    pub session_index: Option<String>,
    pub name_id: Option<String>,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

impl SsoSession {
    /// Creates a new SSO session
    pub fn new(
        user_id: UserId,
        tenant_id: TenantId,
        provider_id: Uuid,
        session_index: Option<String>,
        name_id: Option<String>,
        expires_at: OffsetDateTime,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            tenant_id,
            provider_id,
            session_index,
            name_id,
            created_at: OffsetDateTime::now_utc(),
            expires_at,
        }
    }

    /// Checks if the session is expired
    pub fn is_expired(&self) -> bool {
        OffsetDateTime::now_utc() >= self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[test]
    fn test_sso_provider_creation() {
        let tenant_id = TenantId::new();

        // Test SAML provider creation
        let saml_provider = SsoProvider::new_saml(
            tenant_id,
            "SAML Provider".to_string(),
            Some("Test SAML provider".to_string()),
            Some("https://metadata.url".to_string()),
            None,
            "entity_id".to_string(),
            "https://acs.url".to_string(),
            Some("https://slo.url".to_string()),
        );

        assert_eq!(saml_provider.provider_type, SsoProviderType::Saml);
        assert!(saml_provider.entity_id.is_some());
        assert!(saml_provider.client_id.is_none());

        // Test OIDC provider creation
        let oidc_provider = SsoProvider::new_oidc(
            tenant_id,
            "OIDC Provider".to_string(),
            Some("Test OIDC provider".to_string()),
            "client_id".to_string(),
            "client_secret".to_string(),
            "https://issuer.url".to_string(),
            Some("https://discovery.url".to_string()),
        );

        assert_eq!(oidc_provider.provider_type, SsoProviderType::Oidc);
        assert!(oidc_provider.client_id.is_some());
        assert!(oidc_provider.entity_id.is_none());
    }

    #[test]
    fn test_sso_session_expiration() {
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let provider_id = Uuid::new_v4();

        // Test expired session
        let expired_session = SsoSession::new(
            user_id,
            tenant_id,
            provider_id,
            None,
            None,
            OffsetDateTime::now_utc() - Duration::minutes(1),
        );
        assert!(expired_session.is_expired());

        // Test active session
        let active_session = SsoSession::new(
            user_id,
            tenant_id,
            provider_id,
            None,
            None,
            OffsetDateTime::now_utc() + Duration::hours(1),
        );
        assert!(!active_session.is_expired());
    }

    #[test]
    fn test_sso_user_mapping() {
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let provider_id = Uuid::new_v4();

        let mapping = SsoUserMapping::new(
            user_id,
            tenant_id,
            provider_id,
            "external_id".to_string(),
            "user@example.com".to_string(),
        );

        assert_eq!(mapping.user_id, user_id);
        assert_eq!(mapping.tenant_id, tenant_id);
        assert_eq!(mapping.provider_id, provider_id);
        assert_eq!(mapping.external_id, "external_id");
        assert_eq!(mapping.email, "user@example.com");
    }
}