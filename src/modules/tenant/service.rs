use crate::{
    modules::tenant::{models::Tenant, repository::TenantRepository},
    shared::error::Result,
};
use std::time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

/// Service for tenant management
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
        self.repository.create_tenant(tenant).await
    }

    /// Gets a tenant by ID
    pub async fn get_tenant(&self, id: Uuid) -> Result<Option<Tenant>> {
        self.repository.get_tenant(id).await
    }

    /// Updates a tenant
    pub async fn update_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        self.repository.update_tenant(tenant).await
    }

    /// Lists all tenants
    pub async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        self.repository.list_tenants().await
    }

    /// Deletes a tenant
    pub async fn delete_tenant(&self, id: &str) -> Result<()> {
        let id = uuid::Uuid::parse_str(id).map_err(|e| {
            crate::shared::error::Error::InvalidInput(format!("Invalid UUID: {}", e))
        })?;
        self.repository.delete_tenant(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::database::tests::create_test_db;

    #[tokio::test]
    async fn test_tenant_crud() {
        let (db, _container) = create_test_db().await.unwrap();
        let service = TenantService::new(TenantRepository::new(db.get_pool()));

        // Create test tenant
        let tenant = Tenant::new(
            "Test Tenant".to_string(),
            format!("{}.example.com", Uuid::new_v4()),
        );

        let mut retries = 3;
        let created = loop {
            match service.create_tenant(tenant.clone()).await {
                Ok(t) => break t,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to create tenant: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };

        assert_eq!(created.name, tenant.name);
        assert_eq!(created.domain, tenant.domain);
        assert_eq!(created.active, tenant.active);

        // Test get_tenant
        let retrieved = service.get_tenant(tenant.id.0).await.unwrap().unwrap();
        assert_eq!(retrieved.id, tenant.id);
        assert_eq!(retrieved.name, tenant.name);

        // Test list_tenants
        let tenants = service.list_tenants().await.unwrap();
        assert_eq!(tenants.len(), 1);
        assert_eq!(tenants[0].id, tenant.id);

        // Test update_tenant
        let mut updated_tenant = tenant.clone();
        updated_tenant.name = "Updated Tenant".to_string();
        let updated = service.update_tenant(updated_tenant).await.unwrap();
        assert_eq!(updated.name, "Updated Tenant");

        // Test delete_tenant
        service
            .delete_tenant(&tenant.id.0.to_string())
            .await
            .unwrap();
        let deleted = service.get_tenant(tenant.id.0).await.unwrap();
        assert!(deleted.is_none());
    }
}
