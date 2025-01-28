use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tracing::info;

use crate::core::config::DatabaseConfig;
use crate::shared::{
    error::{Error, Result},
    types::TenantId,
    traits::TenantAware,
};

/// Represents a database connection pool
#[derive(Debug, Clone)]
pub struct Database {
    pool: Pool<Postgres>,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            pool: PgPoolOptions::new()
                .max_connections(5)
                .connect_lazy("postgres://localhost/acci_rust").unwrap(),
        }
    }
}

impl Database {
    /// Creates a new database connection pool
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        let connection_string = format!(
            "postgres://{}:{}@{}:{}/{}",
            config.username,
            config.password,
            config.host,
            config.port,
            config.database
        );

        info!("Connecting to database at {}:{}", config.host, config.port);

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&connection_string)
            .await?;

        // Initialize database with required extensions and RLS policies
        Self::initialize_database(&pool).await?;

        Ok(Self { pool })
    }

    /// Initialize database with required extensions and RLS policies
    async fn initialize_database(pool: &Pool<Postgres>) -> Result<()> {
        // Create extension for UUID support if not exists
        sqlx::query!("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"")
            .execute(pool)
            .await?;

        info!("Database initialized successfully");

        Ok(())
    }

    /// Sets the current tenant context for the database session
    pub async fn set_tenant_context(&self, tenant_id: TenantId) -> Result<()> {
        sqlx::query!(
            "SET app.current_tenant = $1",
            tenant_id.0.to_string()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Clears the current tenant context
    pub async fn clear_tenant_context(&self) -> Result<()> {
        sqlx::query!("RESET app.current_tenant")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Returns the underlying connection pool
    pub fn pool(&self) -> &Pool<Postgres> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_database_initialization() {
        let config = DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "acci_rust_test".to_string(),
            max_connections: 5,
            ssl_mode: false,
        };

        let db = Database::connect(&config).await.unwrap();
        
        // Verify that we can execute queries
        let result = sqlx::query!("SELECT 1 as one")
            .fetch_one(db.pool())
            .await
            .unwrap();
            
        assert_eq!(result.one, Some(1));
    }
}