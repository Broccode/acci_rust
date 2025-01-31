pub mod core;
pub mod modules;
pub mod shared;

pub use modules::{
    identity::{
        models::{User, Role, RoleType, Permission, PermissionAction, Credentials},
        rbac::{PermissionCheck, RequirePermission},
        AuthenticationService,
        IdentityModule,
    },
    tenant::{
        models::Tenant,
        router as tenant_router,
    },
};

pub use shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
};