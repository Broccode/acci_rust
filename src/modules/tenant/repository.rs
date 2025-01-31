use sqlx::PgPool;
use uuid::Uuid;

use super::models::Tenant;
use crate::shared::{
    error::{Error, Result},
    types::TenantId,
};

/// Repository for tenant management
#[derive(Debug, Clone)]
pub struct TenantRepository {
    db: PgPool,
}

impl TenantRepository {
    /// Creates a new TenantRepository instance
    pub fn new(pool: PgPool) -> Self {
        Self { db: pool }
    }

    /// Creates a new tenant
    pub async fn create_tenant(&self, tenant: &Tenant) -> Result<Tenant> {
        let row = sqlx::query!(
            r#"
            INSERT INTO tenants (id, name, domain, active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, name, domain, active, created_at, updated_at
            "#,
            tenant.id.0,
            tenant.name,
            tenant.domain,
            tenant.active,
            tenant.created_at,
            tenant.updated_at,
        )
        .fetch_one(&self.db)
        .await?;

        Ok(Tenant {
            id: TenantId(row.id),
            name: row.name,
            domain: row.domain,
            active: row.active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Gets a tenant by ID
    pub async fn get_tenant_by_id(&self, id: Uuid) -> Result<Tenant> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, domain, active, created_at, updated_at
            FROM tenants
            WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(r) => Ok(Tenant {
                id: TenantId(r.id),
                name: r.name,
                domain: r.domain,
                active: r.active,
                created_at: r.created_at,
                updated_at: r.updated_at,
            }),
            None => Err(Error::NotFound("Tenant not found".into())),
        }
    }

    /// Gets a tenant by domain
    pub async fn get_tenant_by_domain(&self, domain: &str) -> Result<Tenant> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, domain, active, created_at, updated_at
            FROM tenants
            WHERE domain = $1
            "#,
            domain,
        )
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(r) => Ok(Tenant {
                id: TenantId(r.id),
                name: r.name,
                domain: r.domain,
                active: r.active,
                created_at: r.created_at,
                updated_at: r.updated_at,
            }),
            None => Err(Error::NotFound("Tenant not found".into())),
        }
    }

    /// Updates a tenant
    pub async fn update_tenant(&self, tenant: &Tenant) -> Result<Tenant> {
        let row = sqlx::query!(
            r#"
            UPDATE tenants
            SET name = $1, domain = $2, active = $3, updated_at = NOW()
            WHERE id = $4
            RETURNING id, name, domain, active, created_at, updated_at
            "#,
            tenant.name,
            tenant.domain,
            tenant.active,
            tenant.id.0,
        )
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(r) => Ok(Tenant {
                id: TenantId(r.id),
                name: r.name,
                domain: r.domain,
                active: r.active,
                created_at: r.created_at,
                updated_at: r.updated_at,
            }),
            None => Err(Error::NotFound("Tenant not found".into())),
        }
    }

    /// Lists all tenants
    pub async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, name, domain, active, created_at, updated_at
            FROM tenants
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Tenant {
                id: TenantId(r.id),
                name: r.name,
                domain: r.domain,
                active: r.active,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::database::Database;

    #[tokio::test]
    async fn test_tenant_crud() {
        let db = Database::default();
        let repository = TenantRepository::new(db.get_pool());

        // Create tenant
        let tenant = Tenant::new("Test Tenant".to_string());
        let created = repository.create_tenant(&tenant).await.unwrap();
        assert_eq!(created.name, tenant.name);

        // Get tenant
        let retrieved = repository.get_tenant_by_id(created.id.0).await.unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.name, created.name);

        // Update tenant
        let mut updated = retrieved.clone();
        updated.name = "Updated Tenant".to_string();
        let updated = repository.update_tenant(&updated).await.unwrap();
        assert_eq!(updated.name, "Updated Tenant");

        // List tenants
        let tenants = repository.list_tenants().await.unwrap();
        assert!(!tenants.is_empty());
        assert!(tenants.iter().any(|t| t.id == created.id));
    }
}
