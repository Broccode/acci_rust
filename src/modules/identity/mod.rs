pub mod auth;
pub mod models;
pub mod mfa;
pub mod rbac;
pub mod repository;
pub mod service;
pub mod session;
pub mod session_manager;

pub use auth::AuthenticationService;
pub use service::IdentityModule;
pub use session::RedisSessionStore;

use crate::{
    core::database::Database,
    shared::error::Result,
};

/// Creates a new identity module with authentication service
pub async fn create_identity_module(db: Database) -> Result<(IdentityModule, AuthenticationService)> {
    let repository = repository::UserRepository::new(db.get_pool());
    let session_store = RedisSessionStore::new("redis://localhost:6379")?;
    let module = IdentityModule::new(repository.clone());
    let auth_service = AuthenticationService::new(repository, Box::new(session_store));
    Ok((module, auth_service))
}