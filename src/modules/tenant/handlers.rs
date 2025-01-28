use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::shared::{
    error::Result,
    types::TenantId,
};
use super::{
    models::{CreateTenantRequest, UpdateTenantRequest, TenantResponse},
    service::TenantModule,
};

/// Creates a new tenant
#[axum::debug_handler]
pub async fn create_tenant(
    State(tenant_module): State<TenantModule>,
    Json(request): Json<CreateTenantRequest>,
) -> Result<Json<TenantResponse>> {
    let tenant = tenant_module.create_tenant(request).await?;
    Ok(Json(TenantResponse::from(tenant)))
}

/// Retrieves a tenant by ID
#[axum::debug_handler]
pub async fn get_tenant(
    State(tenant_module): State<TenantModule>,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<TenantResponse>> {
    let tenant = tenant_module.get_tenant(TenantId(tenant_id)).await?;
    Ok(Json(TenantResponse::from(tenant)))
}

/// Updates a tenant
#[axum::debug_handler]
pub async fn update_tenant(
    State(tenant_module): State<TenantModule>,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<UpdateTenantRequest>,
) -> Result<Json<TenantResponse>> {
    let tenant = tenant_module
        .update_tenant(TenantId(tenant_id), request)
        .await?;
    Ok(Json(TenantResponse::from(tenant)))
}

/// Lists all tenants
#[axum::debug_handler]
pub async fn list_tenants(
    State(tenant_module): State<TenantModule>,
) -> Result<Json<Vec<TenantResponse>>> {
    let tenants = tenant_module.list_tenants().await?;
    Ok(Json(
        tenants
            .into_iter()
            .map(TenantResponse::from)
            .collect::<Vec<_>>(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    // TODO: Implement handler tests
    // Will need to set up test database and migrations first
}