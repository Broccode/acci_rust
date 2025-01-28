use async_trait::async_trait;
use uuid::Uuid;

use super::types::TenantId;

/// Trait for entities that belong to a specific tenant
pub trait TenantScoped {
    /// Returns the tenant ID that this entity belongs to
    fn tenant_id(&self) -> TenantId;
}

/// Trait for entities that can be uniquely identified
pub trait Identifiable {
    /// Returns the unique identifier of this entity
    fn id(&self) -> Uuid;
}

/// Trait for entities that can be validated
#[async_trait]
pub trait Validatable {
    type Error;

    /// Validates the entity
    async fn validate(&self) -> Result<(), Self::Error>;
}

/// Trait for tenant-aware repositories
#[async_trait]
pub trait TenantAware {
    /// Sets the current tenant context
    async fn set_tenant_context(&self, tenant_id: TenantId) -> crate::shared::error::Result<()>;
    
    /// Clears the current tenant context
    async fn clear_tenant_context(&self) -> crate::shared::error::Result<()>;
}