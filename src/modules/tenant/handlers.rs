use axum::http::StatusCode;
use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    modules::tenant::{
        models::{Tenant, TenantRequest, TenantResponse},
        service::TenantService,
    },
    shared::{error::Result, types::TenantId},
};

/// Creates a new tenant
#[axum::debug_handler]
async fn create_tenant(
    State(service): State<TenantService>,
    Json(request): Json<TenantRequest>,
) -> Result<(StatusCode, Json<TenantResponse>)> {
    let tenant = Tenant::new(request.name);
    let created = service.create_tenant(tenant).await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

/// Gets a tenant by ID
#[axum::debug_handler]
async fn get_tenant(
    State(service): State<TenantService>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<TenantResponse>)> {
    let tenant = service.get_tenant(TenantId(id)).await?;
    Ok((StatusCode::OK, Json(tenant.into())))
}

/// Updates a tenant
#[axum::debug_handler]
async fn update_tenant(
    State(service): State<TenantService>,
    Path(id): Path<Uuid>,
    Json(request): Json<TenantRequest>,
) -> Result<(StatusCode, Json<TenantResponse>)> {
    let mut tenant = service.get_tenant(TenantId(id)).await?;
    tenant.name = request.name;
    let updated = service.update_tenant(tenant).await?;
    Ok((StatusCode::OK, Json(updated.into())))
}

/// Lists all tenants
#[axum::debug_handler]
async fn list_tenants(
    State(service): State<TenantService>,
) -> Result<(StatusCode, Json<Vec<TenantResponse>>)> {
    let tenants = service.list_tenants().await?;
    Ok((
        StatusCode::OK,
        Json(tenants.into_iter().map(Into::into).collect()),
    ))
}

/// Creates the tenant module router
pub fn router(service: TenantService) -> Router {
    Router::new()
        .route("/tenants", post(create_tenant))
        .route("/tenants", get(list_tenants))
        .route("/tenants/:id", get(get_tenant))
        .route("/tenants/:id", put(update_tenant))
        .with_state(service)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::database::Database;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_create_tenant() {
        let db = Database::default();
        let repository = crate::modules::tenant::repository::TenantRepository::new(db.get_pool());
        let service = TenantService::new(repository);
        let app = router(service);

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
        let repository = crate::modules::tenant::repository::TenantRepository::new(db.get_pool());
        let service = TenantService::new(repository);
        let app = router(service);

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
