use std::net::SocketAddr;
use axum::{
    Router,
    routing::get,
    response::IntoResponse,
    http::{StatusCode, Method, HeaderName, HeaderValue},
};
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::core::config::ServerConfig;

/// Server instance
#[derive(Debug)]
pub struct Server {
    config: ServerConfig,
}

impl Server {
    /// Creates a new server instance
    pub async fn new(config: &ServerConfig) -> crate::shared::error::Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    /// Creates the router with all routes
    pub fn create_router(&self) -> Router {
        // Convert allowed methods to Method enum
        let methods = [
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
        ];

        // Convert allowed headers to HeaderName
        let headers = [
            HeaderName::from_static("authorization"),
            HeaderName::from_static("content-type"),
        ];

        // Convert allowed origins to HeaderValue
        let origins: Vec<HeaderValue> = self.config.cors_allowed_origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect();

        Router::new()
            .route("/health", get(health_check))
            .layer(
                CorsLayer::new()
                    .allow_origin(origins)
                    .allow_methods(methods)
                    .allow_headers(headers)
            )
    }

    /// Runs the server
    pub async fn run(&self) -> crate::shared::error::Result<()> {
        let app = self.create_router();

        let addr = SocketAddr::from(([127, 0, 0, 1], self.config.port));
        info!("Server listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| crate::shared::error::Error::Internal(format!("Failed to bind server: {}", e)))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| crate::shared::error::Error::Internal(format!("Server error: {}", e)))?;

        Ok(())
    }
}

/// Health check handler
async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::util::ServiceExt;
    use axum::body::Body;
    use axum::http::Request;

    #[tokio::test]
    async fn test_health_check() {
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_allowed_origins: vec!["http://localhost:3000".to_string()],
        };

        let server = Server::new(&config).await.unwrap();
        let app = server.create_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cors() {
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_allowed_origins: vec!["http://localhost:3000".to_string()],
        };

        let server = Server::new(&config).await.unwrap();
        let app = server.create_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header("Origin", "http://localhost:3000")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();

        assert_eq!(
            response.headers()
                .get("access-control-allow-origin")
                .unwrap(),
            "http://localhost:3000"
        );
    }
}