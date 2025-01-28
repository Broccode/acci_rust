use serde::Deserialize;

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
    pub ssl_mode: bool,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_allowed_origins: Vec<String>,
}

/// Main configuration structure
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
}

impl Config {
    /// Creates a new configuration instance from environment variables
    pub fn from_env() -> crate::shared::error::Result<Self> {
        // TODO: Implement environment variable loading
        // For now, return development defaults
        Ok(Self {
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                username: "postgres".to_string(),
                password: "postgres".to_string(),
                database: "acci_rust".to_string(),
                max_connections: 5,
                ssl_mode: false,
            },
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                cors_allowed_origins: vec!["http://localhost:3000".to_string()],
            },
        })
    }
}