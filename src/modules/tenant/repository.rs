use sqlx::PgPool;
use time::OffsetDateTime;

use crate::core::database::Database;
use crate::shared::{
    error::{Error, Result},
    types::TenantId,
};
use super::models::Tenant;

/// Repository for tenant-related database operations
#[derive(Debug, Clone)] 
pub struct TenantRepository {
    db: Database,
}

impl TenantRepository {
    /// Creates a new TenantRepository instance
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Creates a new tenant in the database
    pub async fn create_tenant(&self, tenant: &Tenant) -> Result<Tenant> {
        let pool = self.db.pool();
        
        sqlx::query!(
            r#"
            INSERT INTO tenants (id, name, created_at, updated_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, created_at, updated_at
            "#,
            tenant.id.0,
            tenant.name,
            tenant.created_at,
            tenant.updated_at,
        )
        .fetch_one(pool)
        .await
        .map(|row| Tenant {
            id: TenantId(row.id),
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .map_err(Error::from)
    }

    /// Retrieves a tenant by their ID
    pub async fn get_tenant_by_id(&self, id: TenantId) -> Result<Tenant> {
        let pool = self.db.pool();
        
        sqlx::query!(
            r#"
            SELECT id, name, created_at, updated_at
            FROM tenants
            WHERE id = $1
            "#,
            id.0,
        )
        .fetch_optional(pool)
        .await?
        .map(|row| Tenant {
            id: TenantId(row.id),
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .ok_or_else(|| Error::NotFound(format!("Tenant not found: {}", id.0)))
    }

    /// Updates a tenant's information
    pub async fn update_tenant(&self, tenant: &Tenant) -> Result<Tenant> {
        let pool = self.db.pool();
        
        sqlx::query!(
            r#"
            UPDATE tenants
            SET name = $1, updated_at = $2
            WHERE id = $3
            RETURNING id, name, created_at, updated_at
            "#,
            tenant.name,
            OffsetDateTime::now_utc(),
            tenant.id.0,
        )
        .fetch_optional(pool)
        .await?
        .map(|row| Tenant {
            id: TenantId(row.id),
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .ok_or_else(|| Error::NotFound(format!("Tenant not found: {}", tenant.id.0)))
    }

    /// Lists all tenants
    pub async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        let pool = self.db.pool();
        
        sqlx::query!(
            r#"
            SELECT id, name, created_at, updated_at
            FROM tenants
            ORDER BY name
            "#,
        )
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| Tenant {
                    id: TenantId(row.id),
                    name: row.name,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                })
                .collect()
        })
        .map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Implement tests using test database
    // Will need to set up test database and migrations first
}