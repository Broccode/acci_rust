use crate::shared::{
    error::Result,
    types::{TenantId, UserId},
};
use super::{
    auth::AuthenticationService,
    models::{Credentials, User},
    repository::UserRepository,
};

/// Main service for identity management
#[derive(Debug)]
pub struct IdentityModule {
    repository: UserRepository,
    auth_service: AuthenticationService,
}

impl IdentityModule {
    /// Creates a new IdentityModule instance
    pub fn new(repository: UserRepository, auth_service: AuthenticationService) -> Self {
        Self {
            repository,
            auth_service,
        }
    }

    /// Authenticates a user with the provided credentials
    pub async fn authenticate(&self, credentials: Credentials) -> Result<User> {
        self.auth_service.authenticate(credentials).await
    }

    /// Creates a new user
    pub async fn create_user(
        &self,
        email: String,
        password: String,
        tenant_id: TenantId,
    ) -> Result<User> {
        self.auth_service
            .create_user(email, password, tenant_id)
            .await
    }

    /// Retrieves a user by their ID
    pub async fn get_user(&self, user_id: UserId, tenant_id: TenantId) -> Result<User> {
        self.repository.set_tenant_context(tenant_id).await?;
        let user = self.repository.get_user_by_id(user_id).await?;
        self.repository.clear_tenant_context().await?;
        Ok(user)
    }

    /// Returns a reference to the user repository
    pub fn repository(&self) -> &UserRepository {
        &self.repository
    }

    /// Returns a reference to the authentication service
    pub fn auth_service(&self) -> &AuthenticationService {
        &self.auth_service
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Implement tests
    // Will need to set up test database and migrations first
}