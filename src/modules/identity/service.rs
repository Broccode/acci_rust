use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
    traits::TenantAware,
};
use super::{
    models::User,
    repository::UserRepository,
};

/// Service for managing user identities
#[derive(Debug)]
pub struct IdentityModule {
    repository: UserRepository,
}

impl IdentityModule {
    /// Creates a new IdentityModule instance
    pub fn new(repository: UserRepository) -> Self {
        Self { repository }
    }

    /// Creates a new user
    pub async fn create_user(&self, user: User) -> Result<User> {
        self.repository.create_user(&user).await
    }

    /// Retrieves a user by their ID
    pub async fn get_user(&self, id: UserId) -> Result<User> {
        self.repository.get_user_by_id(id).await
    }

    /// Returns a reference to the user repository
    pub fn repository(&self) -> &UserRepository {
        &self.repository
    }
}

#[async_trait::async_trait]
impl TenantAware for IdentityModule {
    async fn set_tenant_context(&self, tenant_id: TenantId) -> Result<()> {
        self.repository.set_tenant_context(tenant_id).await
    }

    async fn clear_tenant_context(&self) -> Result<()> {
        self.repository.clear_tenant_context().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Implement tests
    // Will need to set up test database and migrations first
}