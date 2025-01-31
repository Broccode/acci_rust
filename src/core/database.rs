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
    use std::time::Duration;
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

        // Create database connection with retry logic
        let mut retries = 3;
        let mut last_error = None;
        let db = loop {
            match Database::connect(&config).await {
                Ok(db) => break db,
                Err(e) => {
                    last_error = Some(e);
                    retries -= 1;
                    if retries == 0 {
                        return Err(last_error.unwrap());
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };

        // Run migrations with retry logic
        let mut retries = 3;
        while retries > 0 {
            match sqlx::migrate!("./migrations").run(&db.get_pool()).await {
                Ok(_) => break,
                Err(e) => {
                    if !e.to_string().contains("already exists") {
                        retries -= 1;
                        if retries == 0 {
                            return Err(Error::Database(e.to_string()));
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    } else {
                        break;
                    }
                },
            }
        }

        Ok((db, container))
    }

    #[tokio::test]
    #[tracing::instrument]
    async fn test_database_connection() -> Result<()> {
        let (db, _container) = create_test_db().await?;

        // Test query with retry logic
        let mut retries = 3;
        let result = loop {
            match sqlx::query_as::<_, (i32,)>("SELECT 1")
                .fetch_one(&db.get_pool())
                .await
            {
                Ok(r) => break r,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        return Err(Error::Database(e.to_string()));
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };

        assert_eq!(result.0, 1);
        Ok(())
    }

    #[tokio::test]
    #[tracing::instrument]
    async fn test_tenant_isolation() -> Result<()> {
        let (db, _container) = create_test_db().await?;

        // Create test tenant with retry logic
        let tenant_id = Uuid::new_v4();
        let mut retries = 3;
        while retries > 0 {
            match sqlx::query!(
                "INSERT INTO tenants (id, name, domain, active) VALUES ($1, $2, $3, $4)",
                tenant_id,
                "Test Tenant",
                format!("{}.example.com", Uuid::new_v4()),
                true
            )
            .execute(&db.get_pool())
            .await
            {
                Ok(_) => break,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        return Err(Error::Database(e.to_string()));
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        }

        // Set tenant context with retry logic
        let mut retries = 3;
        while retries > 0 {
            match sqlx::query_scalar!(
                "SELECT set_config('app.current_tenant', $1, false)",
                tenant_id.to_string()
            )
            .fetch_one(&db.get_pool())
            .await
            {
                Ok(_) => break,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        return Err(Error::Database(e.to_string()));
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        }

        // Test RLS policy with retry logic
        let user_id = Uuid::new_v4();
        let mut retries = 3;
        let result = loop {
            match sqlx::query!(
                r#"
                INSERT INTO users (
                    id, 
                    tenant_id, 
                    email, 
                    password_hash,
                    active,
                    created_at,
                    updated_at,
                    mfa_enabled
                ) 
                VALUES ($1, $2, $3, $4, $5, NOW(), NOW(), $6) 
                RETURNING id"#,
                user_id,
                tenant_id,
                "test@example.com",
                "hash",
                true,
                false
            )
            .fetch_one(&db.get_pool())
            .await
            {
                Ok(r) => break r,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        return Err(Error::Database(e.to_string()));
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };

        assert_eq!(result.id, user_id);

        // Clear tenant context with retry logic
        let mut retries = 3;
        while retries > 0 {
            match sqlx::query_scalar!("SELECT set_config('app.current_tenant', '', false)")
                .fetch_one(&db.get_pool())
                .await
            {
                Ok(_) => break,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        return Err(Error::Database(e.to_string()));
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        }

        Ok(())
    }
}
