use std::net::SocketAddr;
use axum::{
    Router,
    routing::get,
    response::IntoResponse,
    http::{StatusCode, HeaderValue},
};
use tower_http::{
    trace::TraceLayer,
    cors::CorsLayer,
};
use tracing::info;

use crate::core::config::ServerConfig;
use crate::shared::error::Result;

/// Represents the HTTP server
#[derive(Debug)]
pub struct Server {
    config: ServerConfig,
}

impl Server {
    /// Creates a new server instance
    pub fn new(config: &ServerConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Starts the HTTP server
    pub async fn run(self) -> Result<()> {
        let app = self.create_router();

        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
        info!("Starting server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| crate::shared::error::Error::Internal(e.to_string()))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| crate::shared::error::Error::Internal(e.to_string()))?;

        Ok(())
    }

    /// Creates the application router with all routes
    fn create_router(&self) -> Router {
        // Convert allowed origins to HeaderValues
        let allowed_origins = self.config.cors_allowed_origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect::<Vec<HeaderValue>>();

        Router::new()
            .route("/health", get(health_check))
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
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_allowed_origins: vec!["http://localhost:3000".to_string()],
        };

        let server = Server::new(&config);
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