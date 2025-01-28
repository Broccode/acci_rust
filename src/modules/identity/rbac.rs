use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
};
use super::models::{User, Role, Permission, PermissionAction};

/// Represents a permission check
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PermissionCheck {
    pub resource: String,
    pub action: PermissionAction,
}

impl PermissionCheck {
    /// Creates a new permission check
    pub fn new(resource: impl Into<String>, action: PermissionAction) -> Self {
        Self {
            resource: resource.into(),
            action,
        }
    }
}

/// RBAC service for handling role-based access control
#[derive(Debug)]
pub struct RbacService {
    permissions_cache: moka::sync::Cache<TenantId, HashSet<PermissionCheck>>,
}

impl RbacService {
    /// Creates a new RbacService instance
    pub fn new() -> Self {
        Self {
            permissions_cache: moka::sync::Cache::builder()
                .time_to_live(std::time::Duration::from_secs(300)) // 5 minutes
                .build(),
        }
    }

    /// Checks if a user has a specific permission
    pub fn has_permission(&self, user: &User, check: &PermissionCheck) -> bool {
        user.roles.iter().any(|role| {
            role.permissions.iter().any(|perm| {
                perm.resource == check.resource && 
                match perm.action {
                    PermissionAction::Admin => true, // Admin has all permissions
                    _ => perm.action == check.action,
                }
            })
        })
    }

    /// Checks if a user has all of the specified permissions
    pub fn has_all_permissions(&self, user: &User, checks: &[PermissionCheck]) -> bool {
        checks.iter().all(|check| self.has_permission(user, check))
    }

    /// Checks if a user has any of the specified permissions
    pub fn has_any_permission(&self, user: &User, checks: &[PermissionCheck]) -> bool {
        checks.iter().any(|check| self.has_permission(user, check))
    }

    /// Gets all permissions for a user
    pub fn get_user_permissions(&self, user: &User) -> HashSet<PermissionCheck> {
        let mut permissions = HashSet::new();
        
        for role in &user.roles {
            for perm in &role.permissions {
                permissions.insert(PermissionCheck::new(
                    perm.resource.clone(),
                    perm.action,
                ));
                
                // If user has admin permission for a resource, add all other permissions
                if perm.action == PermissionAction::Admin {
                    permissions.insert(PermissionCheck::new(
                        perm.resource.clone(),
                        PermissionAction::Read,
                    ));
                    permissions.insert(PermissionCheck::new(
                        perm.resource.clone(),
                        PermissionAction::Write,
                    ));
                    permissions.insert(PermissionCheck::new(
                        perm.resource.clone(),
                        PermissionAction::Delete,
                    ));
                }
            }
        }

        permissions
    }

    /// Caches permissions for a tenant
    pub fn cache_tenant_permissions(&self, tenant_id: TenantId, permissions: HashSet<PermissionCheck>) {
        self.permissions_cache.insert(tenant_id, permissions);
    }

    /// Gets cached permissions for a tenant
    pub fn get_cached_tenant_permissions(&self, tenant_id: TenantId) -> Option<HashSet<PermissionCheck>> {
        self.permissions_cache.get(&tenant_id)
    }
}

/// Middleware for checking permissions
pub struct RequirePermission {
    check: PermissionCheck,
    rbac: RbacService,
}

impl RequirePermission {
    /// Creates a new RequirePermission middleware
    pub fn new(resource: impl Into<String>, action: PermissionAction, rbac: RbacService) -> Self {
        Self {
            check: PermissionCheck::new(resource, action),
            rbac,
        }
    }
}

#[async_trait::async_trait]
impl<S> tower::Service<axum::http::Request<S>> for RequirePermission
where
    S: Send + 'static,
{
    type Response = axum::http::Request<S>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<
        Output = Result<Self::Response, Self::Error>
    > + Send>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: axum::http::Request<S>) -> Self::Future {
        let check = self.check.clone();
        let rbac = self.rbac.clone();

        Box::pin(async move {
            // Get user from request extension
            let user = request
                .extensions()
                .get::<User>()
                .ok_or_else(|| Error::Authorization("User not found in request".to_string()))?;

            // Check permission
            if !rbac.has_permission(user, &check) {
                return Err(Error::Authorization(format!(
                    "Missing required permission: {} {}",
                    check.resource,
                    check.action
                )));
            }

            Ok(request)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    fn create_test_user() -> User {
        User {
            id: UserId::new(),
            tenant_id: TenantId::new(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            roles: vec![
                Role {
                    id: Uuid::new_v4(),
                    name: "test_role".to_string(),
                    permissions: vec![
                        Permission {
                            id: Uuid::new_v4(),
                            name: "test_permission".to_string(),
                            resource: "test_resource".to_string(),
                            action: PermissionAction::Read,
                        },
                    ],
                },
            ],
            active: true,
            last_login: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn test_permission_check() {
        let rbac = RbacService::new();
        let user = create_test_user();

        // Test permission that user has
        let check = PermissionCheck::new("test_resource", PermissionAction::Read);
        assert!(rbac.has_permission(&user, &check));

        // Test permission that user doesn't have
        let check = PermissionCheck::new("test_resource", PermissionAction::Write);
        assert!(!rbac.has_permission(&user, &check));
    }

    #[test]
    fn test_admin_permission() {
        let rbac = RbacService::new();
        let mut user = create_test_user();

        // Add admin permission
        user.roles[0].permissions.push(Permission {
            id: Uuid::new_v4(),
            name: "admin_permission".to_string(),
            resource: "test_resource".to_string(),
            action: PermissionAction::Admin,
        });

        // Admin should have all permissions for the resource
        let checks = [
            PermissionCheck::new("test_resource", PermissionAction::Read),
            PermissionCheck::new("test_resource", PermissionAction::Write),
            PermissionCheck::new("test_resource", PermissionAction::Delete),
        ];

        assert!(rbac.has_all_permissions(&user, &checks));
    }

    #[test]
    fn test_permission_caching() {
        let rbac = RbacService::new();
        let tenant_id = TenantId::new();
        let mut permissions = HashSet::new();
        
        permissions.insert(PermissionCheck::new("test_resource", PermissionAction::Read));
        permissions.insert(PermissionCheck::new("test_resource", PermissionAction::Write));

        // Cache permissions
        rbac.cache_tenant_permissions(tenant_id, permissions.clone());

        // Verify cached permissions
        let cached = rbac.get_cached_tenant_permissions(tenant_id).unwrap();
        assert_eq!(cached, permissions);
    }
}