use serde::Deserialize;

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_allowed_origins: Vec<String>,
}

impl ServerConfig {
    /// Creates a default development configuration
    pub fn default_dev() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_allowed_origins: vec!["http://localhost:3000".to_string()],
        }
    }
}

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

impl DatabaseConfig {
    /// Creates a default development configuration
    pub fn default_dev() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "acci_rust".to_string(),
            max_connections: 5,
            ssl_mode: false,
        }
    }
}

/// Redis configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

impl RedisConfig {
    /// Creates a default development configuration
    pub fn default_dev() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
}

impl Config {
    /// Creates a default development configuration
    pub fn default_dev() -> Self {
        Self {
            server: ServerConfig::default_dev(),
            database: DatabaseConfig::default_dev(),
            redis: RedisConfig::default_dev(),
        }
    }

    /// Loads configuration from environment variables
    pub fn from_env() -> Self {
        envy::from_env().expect("Failed to load configuration from environment")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_dev_config() {
        let config = Config::default_dev();
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.database.port, 5432);
        assert_eq!(config.redis.url, "redis://localhost:6379");
    }
}