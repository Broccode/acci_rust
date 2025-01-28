//! Identity management module

mod models;
mod repository;
mod service;
mod auth;

pub use models::{User, Role, Permission, Credentials};
pub use service::IdentityModule;
pub use auth::AuthenticationService;

use crate::core::database::Database;
use crate::shared::error::Result;

/// Creates a new Identity Module instance
pub async fn create_identity_module(db: Database) -> Result<(IdentityModule, AuthenticationService)> {
    let repository = repository::UserRepository::new(db.clone());
    let auth_service = AuthenticationService::new(repository.clone());
    let identity_module = IdentityModule::new(repository);
    Ok((identity_module, auth_service))
}