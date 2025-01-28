use crate::shared::{
    error::Result,
    types::TenantId,
};
use super::{
    models::{Tenant, CreateTenantRequest, UpdateTenantRequest},
    repository::TenantRepository,
};

/// Main service for tenant management
#[derive(Debug, Clone)]
pub struct TenantModule {
    repository: TenantRepository,
}

impl TenantModule {
    /// Creates a new TenantModule instance
    pub fn new(repository: TenantRepository) -> Self {
        Self { repository }
    }

    /// Creates a new tenant
    pub async fn create_tenant(&self, request: CreateTenantRequest) -> Result<Tenant> {
        let tenant = Tenant::new(request.name);
        self.repository.create_tenant(&tenant).await
    }

    /// Retrieves a tenant by their ID
    pub async fn get_tenant(&self, id: TenantId) -> Result<Tenant> {
        self.repository.get_tenant_by_id(id).await
    }

    /// Updates a tenant's information
    pub async fn update_tenant(&self, id: TenantId, request: UpdateTenantRequest) -> Result<Tenant> {
        let mut tenant = self.repository.get_tenant_by_id(id).await?;
        tenant.name = request.name;
        self.repository.update_tenant(&tenant).await
    }

    /// Lists all tenants
    pub async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        self.repository.list_tenants().await
    }

    /// Returns a reference to the tenant repository
    pub fn repository(&self) -> &TenantRepository {
        &self.repository
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Implement tests
    // Will need to set up test database and migrations first
}