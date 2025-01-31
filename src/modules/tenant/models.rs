use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::shared::types::TenantId;

/// Tenant model
#[derive(Debug, Clone)]
pub struct Tenant {
    pub id: TenantId,
    pub name: String,
    pub domain: Option<String>,
    pub active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Tenant {
    /// Creates a new tenant
    pub fn new(name: String) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: TenantId::new(),
            name,
            domain: None,
            active: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Tenant request model
#[derive(Debug, Clone, Deserialize)]
pub struct TenantRequest {
    pub name: String,
}

/// Tenant response model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<Tenant> for TenantResponse {
    fn from(tenant: Tenant) -> Self {
        Self {
            id: tenant.id.0,
            name: tenant.name,
            created_at: tenant.created_at,
            updated_at: tenant.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_creation() {
        let name = "Test Tenant".to_string();
        let tenant = Tenant::new(name.clone());

        assert_eq!(tenant.name, name);
        assert!(tenant.active);
        assert!(tenant.domain.is_none());
    }

    #[test]
    fn test_tenant_response_conversion() {
        let tenant = Tenant::new("Test Tenant".to_string());
        let response = TenantResponse::from(tenant.clone());

        assert_eq!(response.id, tenant.id.0);
        assert_eq!(response.name, tenant.name);
        assert_eq!(response.created_at, tenant.created_at);
        assert_eq!(response.updated_at, tenant.updated_at);
    }
}
