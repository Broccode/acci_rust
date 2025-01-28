//! ACCI Framework modules

pub mod identity;
pub mod tenant;

// Re-export commonly used items
pub use identity::{
    IdentityModule,
    AuthenticationService,
    User,
    Role,
    Permission,
    Credentials,
};
pub use tenant::{
    Tenant,
    TenantModule,
};