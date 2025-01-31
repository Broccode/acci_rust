use crate::shared::error::Error;
use axum::http::StatusCode;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use time;
use uuid::Uuid;

use crate::{
    modules::tenant::{
        models::{Tenant, TenantRequest, TenantResponse},
        service::TenantService,
    },
    shared::{error::Result, types::TenantId},
};

/// Creates a new tenant
pub async fn create_tenant(
    State(service): State<TenantService>,
    Json(request): Json<TenantRequest>,
) -> Result<impl IntoResponse> {
    let tenant = service.create_tenant(request.into()).await?;
    Ok((StatusCode::CREATED, Json(TenantResponse::from(tenant))))
}

/// Gets a tenant by ID
pub async fn get_tenant(
    State(service): State<TenantService>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let id = Uuid::parse_str(&id)
        .map_err(|e| crate::shared::error::Error::InvalidInput(format!("Invalid UUID: {}", e)))?;

    match service.get_tenant(id).await? {
        Some(t) => Ok((StatusCode::OK, Json(t))),
        None => Ok((
            StatusCode::NOT_FOUND,
            Json(Tenant {
                id: TenantId(uuid::Uuid::nil()),
                name: String::new(),
                domain: String::new(),
                active: false,
                created_at: time::OffsetDateTime::now_utc(),
                updated_at: time::OffsetDateTime::now_utc(),
            }),
        )),
    }
}

/// Updates a tenant
pub async fn update_tenant(
    State(service): State<TenantService>,
    Path(id): Path<String>,
    Json(request): Json<TenantRequest>,
) -> Result<impl IntoResponse> {
    let id = Uuid::parse_str(&id)
        .map_err(|e| crate::shared::error::Error::InvalidInput(format!("Invalid UUID: {}", e)))?;

    let mut tenant: Tenant = request.into();
    tenant.id = TenantId(id);

    let updated = service.update_tenant(tenant).await?;
    Ok((StatusCode::OK, Json(TenantResponse::from(updated))))
}

/// Lists all tenants
pub async fn list_tenants(State(service): State<TenantService>) -> Result<impl IntoResponse> {
    let tenants = service.list_tenants().await?;
    Ok((
        StatusCode::OK,
        Json(
            tenants
                .into_iter()
                .map(TenantResponse::from)
                .collect::<Vec<_>>(),
        ),
    ))
}

/// Creates the tenant module router
pub fn router(service: TenantService) -> Router {
    Router::new()
        .route("/tenants", post(create_tenant).get(list_tenants))
        .route("/tenants/:id", get(get_tenant).put(update_tenant))
        .with_state(service)
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
        let repository = crate::modules::tenant::repository::TenantRepository::new(db.get_pool());
        let service = TenantService::new(repository);
        let app = router(service).into_service();

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
        let repository = crate::modules::tenant::repository::TenantRepository::new(db.get_pool());
        let service = TenantService::new(repository);
        let app = router(service).into_service();

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
