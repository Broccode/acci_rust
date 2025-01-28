//! Identity management module

mod models;
mod repository;
mod service;
mod session;
mod session_manager;
mod auth;

pub use models::{User, Role, Permission, Credentials};
pub use service::IdentityModule;
pub use auth::AuthenticationService;
pub use session::{Session, SessionStore, RedisSessionStore};

use crate::core::database::Database;
use crate::shared::error::Result;

/// Creates a new Identity Module instance
pub async fn create_identity_module(db: Database) -> Result<(IdentityModule, AuthenticationService)> {
    let repository = repository::UserRepository::new(db.clone());
    let session_store = Box::new(RedisSessionStore::new("redis://localhost")?);
    let jwt_config = session::JwtConfig {
        secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret_key".to_string()),
        issuer: "acci_rust".to_string(),
        audience: "acci_rust_api".to_string(),
        expiration: time::Duration::minutes(30),
    };
    let auth_service = AuthenticationService::new(repository.clone(), session_store, jwt_config);
    let identity_module = IdentityModule::new(repository);
    Ok((identity_module, auth_service))
}