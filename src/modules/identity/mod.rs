//! Identity and Access Management module

mod auth;
mod models;
mod repository;
mod service;

pub use auth::{AuthenticationService, Credentials};
pub use models::{User, Role, Permission};
pub use service::IdentityModule;

use crate::core::database::Database;
use crate::shared::error::Result;

/// Creates a new Identity Module instance
pub async fn create_identity_module(db: Database) -> Result<IdentityModule> {
    let repository = repository::UserRepository::new(db);
    let auth_service = auth::AuthenticationService::new(repository.clone());

    Ok(IdentityModule::new(repository, auth_service))
}