use moka::sync::Cache;

use crate::{
    modules::identity::models::{Permission, PermissionAction, Role, RoleType, User},
    shared::{
        error::Result,
        types::{TenantId, UserId},
    },
};

/// RBAC service for handling permissions
#[derive(Debug)]
pub struct RbacService {
    permission_cache: Cache<String, bool>,
}

impl Default for RbacService {
    fn default() -> Self {
        Self {
            permission_cache: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(std::time::Duration::from_secs(300))
                .build(),
        }
    }
}

impl RbacService {
    /// Creates a new RbacService instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if a user has a specific permission
    pub async fn check_permission(
        &self,
        user: &User,
        action: PermissionAction,
        resource: &str,
    ) -> Result<bool> {
        let cache_key = format!("{}:{}:{}", user.id.0, action, resource);

        if let Some(has_permission) = self.permission_cache.get(&cache_key) {
            return Ok(has_permission);
        }

        let has_permission = user.roles.iter().any(|role| {
            role.permissions
                .iter()
                .any(|permission| permission.action == action && permission.resource == resource)
        });

        self.permission_cache.insert(cache_key, has_permission);
        Ok(has_permission)
    }

    /// Clears the permission cache for a user
    pub fn clear_user_cache(&self, _user_id: UserId) {
        self.permission_cache.invalidate_all();
    }
}

/// Permission check trait for request handlers
#[async_trait::async_trait]
pub trait PermissionCheck {
    /// Gets the user ID from the request
    fn user_id(&self) -> Option<UserId>;

    /// Gets the tenant ID from the request
    fn tenant_id(&self) -> Option<TenantId>;

    /// Gets the required permission action
    fn required_action(&self) -> PermissionAction;

    /// Gets the required permission resource
    fn required_resource(&self) -> &str;
}

/// Require permission attribute for request handlers
pub struct RequirePermission {
    pub action: PermissionAction,
    pub resource: String,
}

/// Checks if a user has the required permission
pub fn has_permission(user: &User, action: PermissionAction, resource: &str) -> bool {
    user.roles.iter().any(|role| {
        role.permissions
            .iter()
            .any(|permission| permission.action == action && permission.resource == resource)
    })
}

/// Creates a new user role
pub fn create_user_role() -> Role {
    let mut role = Role::new(RoleType::User, "User".to_string());
    role.permissions = vec![
        Permission::new(
            "Create User".to_string(),
            PermissionAction::Create,
            "users".to_string(),
        ),
        Permission::new(
            "Read User".to_string(),
            PermissionAction::Read,
            "users".to_string(),
        ),
    ];
    role
}

/// Creates a new admin role
pub fn create_admin_role() -> Role {
    let mut role = Role::new(RoleType::Admin, "Admin".to_string());
    role.permissions = vec![
        Permission::new(
            "Create User".to_string(),
            PermissionAction::Create,
            "users".to_string(),
        ),
        Permission::new(
            "Read User".to_string(),
            PermissionAction::Read,
            "users".to_string(),
        ),
        Permission::new(
            "Update User".to_string(),
            PermissionAction::Update,
            "users".to_string(),
        ),
        Permission::new(
            "Delete User".to_string(),
            PermissionAction::Delete,
            "users".to_string(),
        ),
    ];
    role
}

/// Creates a new super admin role
pub fn create_super_admin_role() -> Role {
    let mut role = Role::new(RoleType::SuperAdmin, "Super Admin".to_string());
    role.permissions = vec![
        Permission::new("All".to_string(), PermissionAction::Create, "*".to_string()),
        Permission::new("All".to_string(), PermissionAction::Read, "*".to_string()),
        Permission::new("All".to_string(), PermissionAction::Update, "*".to_string()),
        Permission::new("All".to_string(), PermissionAction::Delete, "*".to_string()),
    ];
    role
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::identity::models::{Permission, Role, User};
    use time::OffsetDateTime;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_permission_check() {
        let rbac = RbacService::new();

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

        // Test permission exists
        let has_permission = rbac
            .check_permission(&user, PermissionAction::Create, "users")
            .await
            .unwrap();
        assert!(has_permission);

        // Test permission does not exist
        let has_permission = rbac
            .check_permission(&user, PermissionAction::Delete, "users")
            .await
            .unwrap();
        assert!(!has_permission);

        // Test cache hit
        let has_permission = rbac
            .check_permission(&user, PermissionAction::Create, "users")
            .await
            .unwrap();
        assert!(has_permission);

        // Test cache clear
        rbac.clear_user_cache(user.id);
        let has_permission = rbac
            .check_permission(&user, PermissionAction::Create, "users")
            .await
            .unwrap();
        assert!(has_permission);
    }

    #[test]
    fn test_has_permission() {
        let user = User {
            id: UserId(Uuid::new_v4()),
            tenant_id: TenantId(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            roles: vec![{
                let mut role = Role::new(RoleType::Admin, "Admin".to_string());
                role.permissions = vec![Permission::new(
                    "Create User".to_string(),
                    PermissionAction::Create,
                    "users".to_string(),
                )];
                role
            }],
            last_login: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            active: true,
            mfa_enabled: false,
            mfa_secret: None,
        };

        let has_permission = has_permission(&user, PermissionAction::Create, "users");
        assert!(has_permission);
    }

    #[test]
    fn test_create_user_role() {
        let role = create_user_role();
        assert_eq!(role.role_type, RoleType::User);
        assert_eq!(role.name, "User");
        assert_eq!(role.permissions.len(), 2);
    }

    #[test]
    fn test_create_admin_role() {
        let role = create_admin_role();
        assert_eq!(role.role_type, RoleType::Admin);
        assert_eq!(role.name, "Admin");
        assert_eq!(role.permissions.len(), 4);
    }

    #[test]
    fn test_create_super_admin_role() {
        let role = create_super_admin_role();
        assert_eq!(role.role_type, RoleType::SuperAdmin);
        assert_eq!(role.name, "Super Admin");
        assert_eq!(role.permissions.len(), 4);
    }
}
