use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::shared::types::TenantId;

/// Represents a tenant in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: TenantId,
    pub name: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Tenant {
    /// Creates a new tenant with the given name
    pub fn new(name: String) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: TenantId(Uuid::new_v4()),
            name,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Represents the data needed to create a new tenant
#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
}

/// Represents the data needed to update a tenant
#[derive(Debug, Deserialize)]
pub struct UpdateTenantRequest {
    pub name: String,
}

/// Represents a tenant's details in API responses
#[derive(Debug, Serialize)]
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
    fn test_create_tenant() {
        let name = "Test Tenant".to_string();
        let tenant = Tenant::new(name.clone());

        assert_eq!(tenant.name, name);
        assert!(tenant.created_at <= OffsetDateTime::now_utc());
        assert_eq!(tenant.created_at, tenant.updated_at);
    }

    #[test]
    fn test_tenant_response_conversion() {
        let tenant = Tenant::new("Test Tenant".to_string());
        let tenant_id = tenant.id;
        let response = TenantResponse::from(tenant);

        assert_eq!(response.id, tenant_id.0);
    }
}