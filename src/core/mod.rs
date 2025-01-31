pub mod config;
pub mod database;
pub mod server;

use self::{config::Config, database::Database, server::Server};
use crate::shared::error::Result;

#[derive(Debug)]
pub struct Core {
    pub database: Database,
    pub server: Server,
}

impl Core {
    pub async fn new(config: Config) -> Result<Self> {
        let database = Database::connect(&config.database).await?;
        let server = Server::new(&config.server).await?;
        Ok(Self { database, server })
    }

    pub async fn run(&self) -> Result<()> {
        self.database.execute_query(sqlx::query("SELECT 1")).await?;
        self.server.run().await
    }
}

pub async fn init(db: &Database) -> Result<()> {
    db.execute_query(sqlx::query("SELECT 1")).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use self::config::{DatabaseConfig, RedisConfig, ServerConfig};
    use super::*;

    #[tokio::test]
    async fn test_core_initialization() {
        let config = Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                cors_allowed_origins: vec!["http://localhost:3000".to_string()],
            },
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                username: "postgres".to_string(),
                password: "postgres".to_string(),
                database: "acci_rust_test".to_string(),
                max_connections: 5,
                ssl_mode: false,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
            },
        };

        let core = Core::new(config).await.unwrap();
        let result = core
            .database
            .execute_query(sqlx::query("SELECT 1"))
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);
    }

    #[tokio::test]
    async fn test_init() {
        let config = DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "postgres".to_string(),
            max_connections: 5,
            ssl_mode: false,
        };

        let db = Database::connect(&config).await.unwrap();
        init(&db).await.unwrap();
    }
}
