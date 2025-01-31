use crate::{
    core::database::Database,
    modules::{
        identity::{
            models::{Permission, PermissionAction, Role, RoleType, User},
            rbac::{create_user_role, RbacService},
            repository::UserRepository,
        },
        tenant::models::Tenant,
    },
    shared::{
        error::{Error, Result},
        types::{TenantId, UserId},
    },
};
use time::OffsetDateTime;
use uuid::Uuid;

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
        core::database::tests::create_test_db,
        modules::identity::models::{Permission, Role, RoleType},
        modules::tenant::models::Tenant,
        shared::types::{TenantId, UserId},
    };
    use std::time::Duration;
    use time::OffsetDateTime;
    use uuid::Uuid;

    async fn setup_test_tenant(db: &Database) -> Result<Tenant> {
        let tenant = Tenant::new(
            "Test Tenant".to_string(),
            format!("{}.example.com", Uuid::new_v4()),
        );
        let mut retries = 3;
        while retries > 0 {
            match sqlx::query!(
                r#"INSERT INTO tenants (id, name, domain, active) VALUES ($1, $2, $3, $4)"#,
                tenant.id.0 as uuid::Uuid,
                tenant.name,
                tenant.domain,
                tenant.active
            )
            .execute(&db.get_pool())
            .await
            {
                Ok(_) => break,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        return Err(Error::Database(e.to_string()));
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        }
        Ok(tenant)
    }

    #[tokio::test]
    async fn test_user_management() {
        let (db, _container) = create_test_db().await.unwrap();
        let module = IdentityModule::new(UserRepository::new(db.get_pool()));

        // Create test tenant
        let tenant = setup_test_tenant(&db).await.unwrap();

        // Create user
        let user = User {
            id: UserId::new(),
            tenant_id: tenant.id,
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            roles: vec![create_user_role()],
            active: true,
            last_login: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            mfa_enabled: false,
            mfa_secret: None,
        };

        let mut retries = 3;
        let created = loop {
            match module.create_user(&user).await {
                Ok(u) => break u,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to create user: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };
        assert_eq!(created.email, user.email);

        // Test permission check
        let mut retries = 3;
        let has_permission = loop {
            match module
                .check_permission(&created, PermissionAction::Create, "users")
                .await
            {
                Ok(p) => break p,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to check permission: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };
        assert!(has_permission);

        let mut retries = 3;
        let has_permission = loop {
            match module
                .check_permission(&created, PermissionAction::Delete, "users")
                .await
            {
                Ok(p) => break p,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to check permission: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };
        assert!(!has_permission);
    }
}
