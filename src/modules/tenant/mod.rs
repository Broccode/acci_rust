//! Tenant Management module

mod models;
mod repository;
mod service;
pub mod handlers;

pub use models::Tenant;
pub use service::TenantModule;

use crate::core::database::Database;
use crate::shared::error::Result;

/// Creates a new Tenant Module instance
pub async fn create_tenant_module(db: Database) -> Result<TenantModule> {
    let repository = repository::TenantRepository::new(db);
    Ok(TenantModule::new(repository))
}

impl Default for TenantModule {
    fn default() -> Self {
        Self::new(TenantRepository::new(Database::default()))
    }
}