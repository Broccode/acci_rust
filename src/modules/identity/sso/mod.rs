//! SSO module for handling SAML and OIDC authentication
mod models;
mod saml;
mod oidc;
mod repository;
mod service;

pub use models::{SsoProvider, SsoProviderType, SsoUserMapping, SsoSession};
pub use service::SsoService;

use crate::{
    core::database::Database,
    shared::error::Result,
};

/// Creates a new SSO service
pub async fn create_sso_service(db: Database) -> Result<SsoService> {
    let repository = repository::SsoRepository::new(db);
    Ok(SsoService::new(repository))
}