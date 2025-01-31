use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::shared::types::{TenantId, UserId};

/// User credentials for authentication
#[derive(Debug, Clone)]
pub struct Credentials {
    pub email: String,
    pub password: String,
    pub tenant_id: TenantId,
    pub mfa_code: Option<String>,
}

/// User model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub tenant_id: TenantId,
    pub email: String,
    pub password_hash: String,
    pub roles: Vec<Role>,
    pub active: bool,
    pub last_login: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub mfa_enabled: bool,
    pub mfa_secret: Option<String>,
}

/// Role type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleType {
    User,
    Admin,
    SuperAdmin,
}

impl std::fmt::Display for RoleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleType::User => write!(f, "user"),
            RoleType::Admin => write!(f, "admin"),
            RoleType::SuperAdmin => write!(f, "superadmin"),
        }
    }
}

/// Role model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub role_type: RoleType,
    pub name: String,
    pub permissions: Vec<Permission>,
}

impl Role {
    /// Creates a new role
    pub fn new(role_type: RoleType, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            role_type,
            name,
            permissions: Vec::new(),
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.role_type)
    }
}

/// Permission model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub action: PermissionAction,
    pub resource: String,
}

/// Permission action enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionAction {
    Create,
    Read,
    Update,
    Delete,
    List,
    Execute,
}

impl std::fmt::Display for PermissionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionAction::Create => write!(f, "create"),
            PermissionAction::Read => write!(f, "read"),
            PermissionAction::Update => write!(f, "update"),
            PermissionAction::Delete => write!(f, "delete"),
            PermissionAction::List => write!(f, "list"),
            PermissionAction::Execute => write!(f, "execute"),
        }
    }
}

impl User {
    /// Creates a new user
    pub fn new(tenant_id: TenantId, email: String, password_hash: String) -> Self {
        Self {
            id: UserId::new(),
            tenant_id,
            email,
            password_hash,
            roles: Vec::new(),
            active: true,
            last_login: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            mfa_enabled: false,
            mfa_secret: None,
        }
    }

    /// Enables MFA for the user
    pub fn enable_mfa(&mut self, secret: String) {
        self.mfa_enabled = true;
        self.mfa_secret = Some(secret);
        self.updated_at = OffsetDateTime::now_utc();
    }

    /// Disables MFA for the user
    pub fn disable_mfa(&mut self) {
        self.mfa_enabled = false;
        self.mfa_secret = None;
        self.updated_at = OffsetDateTime::now_utc();
    }
}

impl Permission {
    /// Creates a new permission
    pub fn new(name: String, action: PermissionAction, resource: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            action,
            resource,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let tenant_id = TenantId::new();
        let email = "test@example.com".to_string();
        let password_hash = "hash".to_string();

        let user = User::new(tenant_id, email.clone(), password_hash.clone());

        assert_eq!(user.email, email);
        assert_eq!(user.password_hash, password_hash);
        assert_eq!(user.tenant_id, tenant_id);
        assert!(user.active);
        assert!(user.roles.is_empty());
        assert!(user.last_login.is_none());
        assert!(!user.mfa_enabled);
        assert!(user.mfa_secret.is_none());
    }

    #[test]
    fn test_mfa_management() {
        let mut user = User::new(
            TenantId::new(),
            "test@example.com".to_string(),
            "hash".to_string(),
        );

        // Enable MFA
        let secret = "ABCDEFGHIJKLMNOP".to_string();
        user.enable_mfa(secret.clone());
        assert!(user.mfa_enabled);
        assert_eq!(user.mfa_secret, Some(secret));

        // Disable MFA
        user.disable_mfa();
        assert!(!user.mfa_enabled);
        assert!(user.mfa_secret.is_none());
    }

    #[test]
    fn test_role_creation() {
        let role_type = RoleType::Admin;
        let name = "Admin".to_string();
        let role = Role::new(role_type, name.clone());

        assert_eq!(role.role_type, role_type);
        assert_eq!(role.name, name);
        assert!(role.permissions.is_empty());
    }

    #[test]
    fn test_permission_creation() {
        let name = "Create User".to_string();
        let action = PermissionAction::Create;
        let resource = "users".to_string();

        let permission = Permission::new(name.clone(), action, resource.clone());

        assert_eq!(permission.name, name);
        assert_eq!(permission.action, action);
        assert_eq!(permission.resource, resource);
    }

    #[test]
    fn test_permission_action_display() {
        assert_eq!(PermissionAction::Create.to_string(), "create");
        assert_eq!(PermissionAction::Read.to_string(), "read");
        assert_eq!(PermissionAction::Update.to_string(), "update");
        assert_eq!(PermissionAction::Delete.to_string(), "delete");
        assert_eq!(PermissionAction::List.to_string(), "list");
        assert_eq!(PermissionAction::Execute.to_string(), "execute");
    }
}
