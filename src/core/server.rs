use std::net::SocketAddr;
use axum::{
    Router,
    routing::{get, post, put},
    response::IntoResponse,
    http::{StatusCode, HeaderValue},
};
use tower_http::{
    trace::TraceLayer,
    cors::CorsLayer,
};
use tracing::info;

use crate::core::{
    config::ServerConfig,
    database::Database,
};
use crate::shared::error::{Error, Result};
use crate::modules::tenant::{
    handlers as tenant_handlers,
    create_tenant_module,
    TenantModule,
};

/// Represents the HTTP server
#[derive(Debug)]
pub struct Server {
    config: ServerConfig,
    tenant_module: TenantModule,
}

impl Server {
    /// Creates a new server instance
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        let db = Database::connect(&config.database).await?;
        let tenant_module = create_tenant_module(db).await?;
        Ok(Self {
            config: config.clone(),
            tenant_module,
        })
    }

    /// Starts the HTTP server
    pub async fn run(self) -> Result<()> {
        let app = self.create_router();

        let addr = SocketAddr::new(
            self.config.server.host.parse()
                .map_err(|e| Error::Internal(format!("Invalid host address: {}", e)))?,
            self.config.server.port,
        );

        info!("Starting server on http://{}:{}", self.config.server.host, self.config.server.port);

        axum::serve(
            tokio::net::TcpListener::bind(&addr).await
                .map_err(|e| Error::Internal(e.to_string()))?,
            app,
        )
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(())
    }

    /// Creates the application router with all routes
    fn create_router(&self) -> Router {
        // Convert allowed origins to HeaderValues
        let allowed_origins = self.config.server.cors_allowed_origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect::<Vec<HeaderValue>>();

        Router::new()
            .route("/health", get(health_check))
            // Tenant routes
            .route("/api/v1/tenants", post(tenant_handlers::create_tenant))
            .route("/api/v1/tenants", get(tenant_handlers::list_tenants))
            .route("/api/v1/tenants/:id", get(tenant_handlers::get_tenant))
            .route("/api/v1/tenants/:id", put(tenant_handlers::update_tenant))
            .with_state(self.tenant_module.clone())
            // CORS and tracing layers
            .layer(TraceLayer::new_for_http())
            .layer(
                CorsLayer::new()
                    .allow_origin(allowed_origins)
                    .allow_methods([
                        axum::http::Method::GET,
                        axum::http::Method::POST,
                        axum::http::Method::PUT,
                        axum::http::Method::DELETE,
                    ])
                    .allow_headers([
                        axum::http::header::CONTENT_TYPE,
                        axum::http::header::AUTHORIZATION,
                    ])
            )
    }
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn test_health_check() {
        let config = ServerConfig::default_dev();
        let server = Server::new(&config).await.unwrap();
        let app = server.create_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}