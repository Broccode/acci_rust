use crate::{
    modules::identity::{
        models::{PermissionAction, User},
        rbac::RbacService,
        repository::UserRepository,
    },
    shared::{
        error::Result,
        types::{TenantId, UserId},
    },
};

/// Identity module for managing users and permissions
#[derive(Debug)]
pub struct IdentityModule {
    repository: UserRepository,
    rbac: RbacService,
}

impl IdentityModule {
    /// Creates a new IdentityModule instance
    pub fn new(repository: UserRepository) -> Self {
        Self {
            repository,
            rbac: RbacService::new(),
        }
    }

    /// Creates a new user
    pub async fn create_user(&self, user: &User) -> Result<User> {
        self.repository.create_user(user.clone()).await
    }

    /// Gets a user by ID
    pub async fn get_user(&self, id: &str) -> Result<Option<User>> {
        let user_id = UserId(uuid::Uuid::parse_str(id).map_err(|e| {
            crate::shared::error::Error::InvalidInput(format!("Invalid UUID: {}", e))
        })?);
        self.repository.get_user_by_id(user_id).await
    }

    /// Updates a user
    pub async fn update_user(&self, user: &User) -> Result<User> {
        self.repository.update_user(user.clone()).await
    }

    /// Deletes a user
    pub async fn delete_user(&self, id: &str, tenant_id: &str) -> Result<()> {
        let user_id = UserId(uuid::Uuid::parse_str(id).map_err(|e| {
            crate::shared::error::Error::InvalidInput(format!("Invalid UUID: {}", e))
        })?);
        let tenant_id = TenantId(uuid::Uuid::parse_str(tenant_id).map_err(|e| {
            crate::shared::error::Error::InvalidInput(format!("Invalid UUID: {}", e))
        })?);
        self.repository.delete_user(user_id, tenant_id).await
    }

    /// Lists all users
    pub async fn list_users(&self) -> Result<Vec<User>> {
        self.repository.list_users().await
    }

    /// Checks if a user has a specific permission
    pub async fn check_permission(
        &self,
        user: &User,
        action: PermissionAction,
        resource: &str,
    ) -> Result<bool> {
        self.rbac.check_permission(user, action, resource).await
    }
}

impl Default for IdentityModule {
    fn default() -> Self {
        Self {
            repository: UserRepository::default(),
            rbac: RbacService::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        modules::identity::models::{Permission, Role, RoleType},
        shared::types::{TenantId, UserId},
    };
    use time::OffsetDateTime;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_user_management() {
        let module = IdentityModule::default();

        // Create user
        let user = User {
            id: UserId::new(),
            tenant_id: TenantId::new(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            roles: vec![{
                let mut role = Role::new(RoleType::Admin, "Admin".to_string());
                role.permissions = vec![Permission {
                    id: Uuid::new_v4(),
                    name: "Create User".to_string(),
                    action: PermissionAction::Create,
                    resource: "users".to_string(),
                }];
                role
            }],
            active: true,
            last_login: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            mfa_enabled: false,
            mfa_secret: None,
        };

        let created = module.create_user(&user).await.unwrap();
        assert_eq!(created.email, user.email);

        // Test permission check
        let has_permission = module
            .check_permission(&created, PermissionAction::Create, "users")
            .await
            .unwrap();
        assert!(has_permission);

        let has_permission = module
            .check_permission(&created, PermissionAction::Delete, "users")
            .await
            .unwrap();
        assert!(!has_permission);
    }
}
