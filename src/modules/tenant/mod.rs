mod handlers;
pub mod models;
pub mod repository;
pub mod service;

use crate::{core::database::Database, shared::error::Result, shared::types::TenantId};
use axum::Router;

/// Tenant module for managing tenants
#[derive(Debug, Clone)]
pub struct TenantModule {
    service: service::TenantService,
}

impl TenantModule {
    /// Creates a new tenant module
    pub fn new(db: Database) -> Self {
        Self {
            service: service::TenantService::new(repository::TenantRepository::new(db.get_pool())),
        }
    }

    /// Gets the router for this module
    pub fn router(&self) -> Result<Router> {
        Ok(handlers::router(self.service.clone()))
    }
}

/// Creates a router for the tenant module
pub fn router(db: Database) -> Result<Router> {
    let module = TenantModule::new(db);
    module.router()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::database::tests::create_test_db;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_create_tenant() -> Result<()> {
        let (db, _container) = create_test_db().await?;
        let app = router(db)?.into_service();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tenants")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "Test Tenant",
                            "domain": "test.example.com"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_tenant() -> Result<()> {
        let (db, _container) = create_test_db().await?;
        let app = router(db)?.into_service();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/tenants/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }
}
