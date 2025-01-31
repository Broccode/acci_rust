use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::info;

use crate::{
    core::config::DatabaseConfig,
    shared::{
        error::{Error, Result},
        traits::TenantAware,
        types::TenantId,
    },
};

/// Database connection pool
#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Creates a new database connection pool
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        let connection_string = format!(
            "postgres://{}:{}@{}:{}/{}",
            config.username, config.password, config.host, config.port, config.database
        );

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&connection_string)
            .await
            .map_err(|e| Error::Database(format!("Failed to connect to database: {}", e)))?;

        info!("Connected to database");

        Ok(Self { pool })
    }

    /// Gets a clone of the connection pool
    pub fn get_pool(&self) -> PgPool {
        self.pool.clone()
    }

    /// Executes a query using the pool
    pub async fn execute_query<'q>(
        &self,
        query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    ) -> Result<sqlx::postgres::PgQueryResult> {
        query
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))
    }
}

#[async_trait::async_trait]
impl TenantAware for Database {
    async fn set_tenant_context(&self, tenant_id: TenantId) -> Result<()> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| Error::Database(format!("Failed to acquire connection: {}", e)))?;

        sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(tenant_id.0.to_string())
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::Database(format!("Failed to set tenant: {}", e)))?;

        Ok(())
    }

    async fn clear_tenant_context(&self) -> Result<()> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| Error::Database(format!("Failed to acquire connection: {}", e)))?;

        sqlx::query("SELECT set_config('app.current_tenant', '', true)")
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::Database(format!("Failed to clear tenant: {}", e)))?;

        Ok(())
    }
}

impl Default for Database {
    fn default() -> Self {
        Self {
            pool: PgPool::connect_lazy("postgres://postgres:postgres@localhost:5432/acci_rust")
                .expect("Failed to create default database pool"),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Arc;
    use testcontainers::*;
    use testcontainers_modules::postgres::Postgres;
    use uuid::Uuid;

    static DOCKER: Lazy<Arc<clients::Cli>> = Lazy::new(|| Arc::new(clients::Cli::default()));

    pub async fn create_test_db() -> Result<(Database, Container<'static, Postgres>)> {
        let postgres = Postgres::default();
        let container = DOCKER.run(postgres);
        let port = container.get_host_port_ipv4(5432);

        let config = DatabaseConfig {
            host: "localhost".to_string(),
            port,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "postgres".to_string(),
            max_connections: 5,
            ssl_mode: false,
        };

        // Create database connection
        let db = Database::connect(&config).await?;

        // Run migrations, ignore if tables already exist
        match sqlx::migrate!("./migrations").run(&db.get_pool()).await {
            Ok(_) => (),
            Err(e) => {
                if !e.to_string().contains("already exists") {
                    return Err(Error::Database(e.to_string()));
                }
            },
        }

        Ok((db, container))
    }

    #[tokio::test]
    #[tracing::instrument]
    async fn test_database_connection() -> Result<()> {
        let (db, _container) = create_test_db().await?;

        // Test query
        let result: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&db.get_pool()).await?;

        assert_eq!(result.0, 1);
        Ok(())
    }

    #[tokio::test]
    #[tracing::instrument]
    async fn test_tenant_isolation() -> Result<()> {
        let (db, _container) = create_test_db().await?;

        // Create test tenant
        let tenant_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO tenants (id, name) VALUES ($1, $2)",
            tenant_id,
            "Test Tenant"
        )
        .execute(&db.get_pool())
        .await?;

        // Set tenant context
        sqlx::query_scalar!(
            "SELECT set_config('app.current_tenant', $1, false)",
            tenant_id.to_string()
        )
        .fetch_one(&db.get_pool())
        .await?;

        // Test RLS policy
        let result = sqlx::query!(
            "INSERT INTO users (tenant_id, email, password_hash) VALUES ($1, $2, $3) RETURNING id",
            tenant_id,
            "test@example.com",
            "hash"
        )
        .fetch_one(&db.get_pool())
        .await?;

        assert!(result.id != Uuid::nil());

        // Clear tenant context
        sqlx::query_scalar!("SELECT set_config('app.current_tenant', '', false)")
            .fetch_one(&db.get_pool())
            .await?;

        Ok(())
    }
}
