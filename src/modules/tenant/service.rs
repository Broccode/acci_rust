use super::{models::Tenant, repository::TenantRepository};
use crate::shared::{error::Result, types::TenantId};

/// Service for managing tenants
#[derive(Debug, Clone)]
pub struct TenantService {
    repository: TenantRepository,
}

impl TenantService {
    /// Creates a new TenantService instance
    pub fn new(repository: TenantRepository) -> Self {
        Self { repository }
    }

    /// Creates a new tenant
    pub async fn create_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        self.repository.create_tenant(&tenant).await
    }

    /// Gets a tenant by ID
    pub async fn get_tenant(&self, id: TenantId) -> Result<Tenant> {
        self.repository.get_tenant_by_id(id.0).await
    }

    /// Updates a tenant
    pub async fn update_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        let mut current = self.repository.get_tenant_by_id(tenant.id.0).await?;

        current.name = tenant.name;
        current.domain = tenant.domain;
        current.active = tenant.active;

        self.repository.update_tenant(&current).await
    }

    /// Lists all tenants
    pub async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        self.repository.list_tenants().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::database::Database;

    #[tokio::test]
    async fn test_tenant_crud() {
        let db = Database::default();
        let repository = TenantRepository::new(db.get_pool());
        let service = TenantService::new(repository);

        // Create tenant
        let tenant = Tenant::new("Test Tenant".to_string());
        let created = service.create_tenant(tenant.clone()).await.unwrap();
        assert_eq!(created.name, tenant.name);

        // Get tenant
        let retrieved = service.get_tenant(created.id).await.unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.name, created.name);

        // Update tenant
        let mut updated = retrieved.clone();
        updated.name = "Updated Tenant".to_string();
        let updated = service.update_tenant(updated).await.unwrap();
        assert_eq!(updated.name, "Updated Tenant");

        // List tenants
        let tenants = service.list_tenants().await.unwrap();
        assert!(!tenants.is_empty());
        assert!(tenants.iter().any(|t| t.id == created.id));
    }
}
