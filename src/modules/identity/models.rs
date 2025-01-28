use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::shared::types::{TenantId, UserId};
use crate::shared::traits::TenantScoped;

/// Represents a user in the system
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
}

impl TenantScoped for User {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}

/// Represents a role in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub permissions: Vec<Permission>,
}

/// Represents a permission in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub resource: String,
    pub action: PermissionAction,
}

/// Represents possible actions for a permission
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum PermissionAction {
    #[default]
    Read,
    Write,
    Delete,
    Admin,
}

/// Represents user credentials for authentication
#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
    pub tenant_id: TenantId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_action_default() {
        assert_eq!(PermissionAction::default(), PermissionAction::Read);
    }

    #[test]
    fn test_user_tenant_scoped() {
        let tenant_id = TenantId::new();
        let user = User {
            id: UserId::new(),
            tenant_id,
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            roles: vec![],
            active: true,
            last_login: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        };

        assert_eq!(user.tenant_id(), tenant_id);
    }
}