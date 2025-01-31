mod handlers;
pub mod models;
pub mod repository;
pub mod service;

use crate::{core::database::Database, shared::error::Result, shared::types::TenantId};
use axum::Router;

/// Tenant module for managing tenants
#[derive(Debug, Clone)]
pub struct TenantModule {
    db: Database,
}

impl TenantModule {
    /// Creates a new tenant module
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Creates the tenant module router
    pub fn router(&self) -> Result<Router> {
        let repository = repository::TenantRepository::new(self.db.get_pool());
        let service = service::TenantService::new(repository);
        Ok(handlers::router(service))
    }
}

/// Creates the tenant module router
pub fn router(db: Database) -> Result<Router> {
    let module = TenantModule::new(db);
    module.router()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use tower::ServiceExt;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_create_tenant() {
        let db = Database::default();
        let module = TenantModule::new(db);
        let app = module.router().unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tenants")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "Test Tenant",
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_get_tenant() {
        let db = Database::default();
        let module = TenantModule::new(db);
        let app = module.router().unwrap();

        let tenant_id = TenantId(Uuid::new_v4());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/tenants/{}", tenant_id.0))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
