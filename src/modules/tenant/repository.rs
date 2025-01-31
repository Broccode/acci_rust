use sqlx::{Pool, Postgres as PgPool};
use std::time::Duration;
use time::{OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

use crate::{
    core::database::Database,
    modules::tenant::models::Tenant,
    shared::{
        error::{Error, Result},
        types::TenantId,
    },
};

/// Helper function to convert Option<OffsetDateTime> to Option<PrimitiveDateTime>
fn to_primitive_datetime(dt: OffsetDateTime) -> PrimitiveDateTime {
    PrimitiveDateTime::new(dt.date(), dt.time())
}

/// Helper function to convert Option<PrimitiveDateTime> to Option<OffsetDateTime>
fn to_offset_datetime(dt: PrimitiveDateTime) -> OffsetDateTime {
    dt.assume_utc()
}

/// Repository for tenant management
#[derive(Debug, Clone)]
pub struct TenantRepository {
    pool: Pool<PgPool>,
}

impl TenantRepository {
    /// Creates a new TenantRepository instance
    pub fn new(pool: Pool<PgPool>) -> Self {
        Self { pool }
    }

    /// Creates a new tenant
    pub async fn create_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        let row = sqlx::query!(
            r#"
            INSERT INTO tenants (id, name, domain, active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, name, domain, active, created_at, updated_at
            "#,
            tenant.id.0 as uuid::Uuid,
            tenant.name,
            tenant.domain,
            tenant.active,
            to_primitive_datetime(tenant.created_at),
            to_primitive_datetime(tenant.updated_at),
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Tenant {
            id: tenant.id,
            name: row.name,
            domain: row.domain.expect("Domain should not be null"),
            active: row.active,
            created_at: to_offset_datetime(row.created_at),
            updated_at: to_offset_datetime(row.updated_at),
        })
    }

    /// Gets a tenant by ID
    pub async fn get_tenant(&self, id: uuid::Uuid) -> Result<Option<Tenant>> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, domain, active, created_at, updated_at
            FROM tenants
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Tenant {
            id: TenantId(r.id),
            name: r.name,
            domain: r.domain.expect("Domain should not be null"),
            active: r.active,
            created_at: to_offset_datetime(r.created_at),
            updated_at: to_offset_datetime(r.updated_at),
        }))
    }

    /// Gets a tenant by domain
    pub async fn get_tenant_by_domain(&self, domain: &str) -> Result<Tenant> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, domain, active, created_at, updated_at
            FROM tenants
            WHERE domain = $1
            "#,
            domain
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Tenant {
            id: TenantId(row.id),
            name: row.name,
            domain: row.domain.expect("Domain should not be null"),
            active: row.active,
            created_at: to_offset_datetime(row.created_at),
            updated_at: to_offset_datetime(row.updated_at),
        })
    }

    /// Updates a tenant
    pub async fn update_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        let row = sqlx::query!(
            r#"
            UPDATE tenants
            SET name = $1, domain = $2, active = $3, updated_at = $4
            WHERE id = $5
            RETURNING id, name, domain, active, created_at, updated_at
            "#,
            tenant.name,
            tenant.domain,
            tenant.active,
            to_primitive_datetime(tenant.updated_at),
            tenant.id.0 as uuid::Uuid,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Tenant {
            id: tenant.id,
            name: row.name,
            domain: row.domain.expect("Domain should not be null"),
            active: row.active,
            created_at: to_offset_datetime(row.created_at),
            updated_at: to_offset_datetime(row.updated_at),
        })
    }

    /// Lists all tenants
    pub async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, name, domain, active, created_at, updated_at
            FROM tenants
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Tenant {
                id: TenantId(r.id),
                name: r.name,
                domain: r.domain.expect("Domain should not be null"),
                active: r.active,
                created_at: to_offset_datetime(r.created_at),
                updated_at: to_offset_datetime(r.updated_at),
            })
            .collect())
    }

    /// Deletes a tenant
    pub async fn delete_tenant(&self, id: uuid::Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM tenants
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

impl Default for TenantRepository {
    fn default() -> Self {
        Self::new(Database::default().get_pool())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::database::tests::create_test_db;

    #[tokio::test]
    async fn test_tenant_crud() {
        let (db, _container) = create_test_db().await.unwrap();
        let repository = TenantRepository::new(db.get_pool());

        // Create test tenant
        let tenant = Tenant {
            id: TenantId(Uuid::new_v4()),
            name: "Test Tenant".to_string(),
            domain: format!("{}.example.com", Uuid::new_v4()),
            active: true,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        };

        let mut retries = 3;
        let created = loop {
            match repository.create_tenant(tenant.clone()).await {
                Ok(t) => break t,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to create tenant: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };

        assert_eq!(created.name, tenant.name);
        assert_eq!(created.domain, tenant.domain);
        assert_eq!(created.active, tenant.active);

        // Test get_tenant
        let retrieved = repository.get_tenant(tenant.id.0).await.unwrap().unwrap();
        assert_eq!(retrieved.id, tenant.id);
        assert_eq!(retrieved.name, tenant.name);

        // Test list_tenants
        let tenants = repository.list_tenants().await.unwrap();
        assert_eq!(tenants.len(), 1);
        assert_eq!(tenants[0].id, tenant.id);

        // Test update_tenant
        let mut updated_tenant = tenant.clone();
        updated_tenant.name = "Updated Tenant".to_string();
        let updated = repository.update_tenant(updated_tenant).await.unwrap();
        assert_eq!(updated.name, "Updated Tenant");

        // Test delete_tenant
        repository.delete_tenant(tenant.id.0).await.unwrap();
        let deleted = repository.get_tenant(tenant.id.0).await.unwrap();
        assert!(deleted.is_none());
    }
}
