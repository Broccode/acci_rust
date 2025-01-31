use serde_json;
use sqlx::{Pool, Postgres};
use time::{OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

use crate::{
    core::database::Database,
    modules::identity::models::{Role, RoleType, User},
    shared::{
        error::{Error, Result},
        types::{TenantId, UserId},
    },
};

/// Helper function to convert database roles to domain roles
fn convert_roles(roles: Option<Vec<String>>) -> Vec<Role> {
    match roles {
        Some(roles) => roles
            .into_iter()
            .filter_map(|r| serde_json::from_str(&r).ok())
            .collect(),
        None => Vec::new(),
    }
}

/// Helper function to convert domain roles to database roles
fn roles_to_strings(roles: &[Role]) -> Vec<String> {
    roles
        .iter()
        .filter_map(|r| serde_json::to_string(r).ok())
        .collect()
}

/// Helper function to convert Option<OffsetDateTime> to Option<PrimitiveDateTime>
fn to_primitive_datetime(dt: OffsetDateTime) -> PrimitiveDateTime {
    PrimitiveDateTime::new(dt.date(), dt.time())
}

/// Helper function to convert Option<PrimitiveDateTime> to Option<OffsetDateTime>
fn to_offset_datetime(dt: PrimitiveDateTime) -> OffsetDateTime {
    dt.assume_utc()
}

/// Helper function to convert Option<OffsetDateTime> to Option<PrimitiveDateTime>
fn convert_to_primitive(dt: Option<OffsetDateTime>) -> Option<PrimitiveDateTime> {
    dt.map(to_primitive_datetime)
}

/// Helper function to convert Option<PrimitiveDateTime> to Option<OffsetDateTime>
fn convert_to_offset(dt: Option<PrimitiveDateTime>) -> Option<OffsetDateTime> {
    dt.map(to_offset_datetime)
}

/// User repository for database operations
#[derive(Debug, Clone)]
pub struct UserRepository {
    pool: Pool<Postgres>,
}

impl UserRepository {
    /// Creates a new UserRepository instance
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub fn get_pool(&self) -> &Pool<Postgres> {
        &self.pool
    }

    /// Gets a user by email and tenant ID
    pub async fn get_user_by_email(
        &self,
        email: &str,
        tenant_id: TenantId,
    ) -> Result<Option<User>> {
        let result = sqlx::query!(
            r#"
            SELECT id, tenant_id, email, password_hash, active, roles, last_login, created_at, updated_at, mfa_enabled, mfa_secret
            FROM users
            WHERE email = $1 AND tenant_id = $2
            "#,
            email,
            tenant_id.0 as uuid::Uuid,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| User {
            id: UserId(r.id),
            tenant_id: TenantId(r.tenant_id),
            email: r.email,
            password_hash: r.password_hash,
            active: r.active,
            roles: convert_roles(Some(r.roles)),
            last_login: convert_to_offset(r.last_login),
            created_at: to_offset_datetime(r.created_at),
            updated_at: to_offset_datetime(r.updated_at),
            mfa_enabled: r.mfa_enabled,
            mfa_secret: r.mfa_secret,
        }))
    }

    /// Updates a user's last login time
    pub async fn update_last_login(&self, user_id: UserId) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE users
            SET last_login = NOW()
            WHERE id = $1
            "#,
            user_id.0 as uuid::Uuid,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Creates a new user
    pub async fn create_user(&self, user: User) -> Result<User> {
        let result = sqlx::query!(
            r#"
            INSERT INTO users (id, tenant_id, email, password_hash, active, roles, created_at, updated_at, mfa_enabled, mfa_secret)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, tenant_id, email, password_hash, active, roles, last_login, created_at, updated_at, mfa_enabled, mfa_secret
            "#,
            user.id.0 as uuid::Uuid,
            user.tenant_id.0 as uuid::Uuid,
            user.email,
            user.password_hash,
            user.active,
            &roles_to_strings(&user.roles),
            to_primitive_datetime(user.created_at),
            to_primitive_datetime(user.updated_at),
            user.mfa_enabled,
            user.mfa_secret,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: UserId(result.id),
            tenant_id: TenantId(result.tenant_id),
            email: result.email,
            password_hash: result.password_hash,
            active: result.active,
            roles: convert_roles(Some(result.roles)),
            last_login: convert_to_offset(result.last_login),
            created_at: to_offset_datetime(result.created_at),
            updated_at: to_offset_datetime(result.updated_at),
            mfa_enabled: result.mfa_enabled,
            mfa_secret: result.mfa_secret,
        })
    }

    /// Gets a user by ID
    pub async fn get_user_by_id(&self, id: UserId) -> Result<Option<User>> {
        let result = sqlx::query!(
            r#"
            SELECT id, tenant_id, email, password_hash, active, roles, last_login, created_at, updated_at, mfa_enabled, mfa_secret
            FROM users
            WHERE id = $1
            "#,
            id.0 as uuid::Uuid,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| User {
            id: UserId(r.id),
            tenant_id: TenantId(r.tenant_id),
            email: r.email,
            password_hash: r.password_hash,
            active: r.active,
            roles: convert_roles(Some(r.roles)),
            last_login: convert_to_offset(r.last_login),
            created_at: to_offset_datetime(r.created_at),
            updated_at: to_offset_datetime(r.updated_at),
            mfa_enabled: r.mfa_enabled,
            mfa_secret: r.mfa_secret,
        }))
    }

    /// Updates a user
    pub async fn update_user(&self, user: User) -> Result<User> {
        let result = sqlx::query!(
            r#"
            UPDATE users
            SET email = $1, password_hash = $2, active = $3, roles = $4, updated_at = $5, mfa_enabled = $6, mfa_secret = $7
            WHERE id = $8 AND tenant_id = $9
            RETURNING id, tenant_id, email, password_hash, active, roles, last_login, created_at, updated_at, mfa_enabled, mfa_secret
            "#,
            user.email,
            user.password_hash,
            user.active,
            &roles_to_strings(&user.roles),
            to_primitive_datetime(user.updated_at),
            user.mfa_enabled,
            user.mfa_secret,
            user.id.0 as uuid::Uuid,
            user.tenant_id.0 as uuid::Uuid,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: UserId(result.id),
            tenant_id: TenantId(result.tenant_id),
            email: result.email,
            password_hash: result.password_hash,
            active: result.active,
            roles: convert_roles(Some(result.roles)),
            last_login: convert_to_offset(result.last_login),
            created_at: to_offset_datetime(result.created_at),
            updated_at: to_offset_datetime(result.updated_at),
            mfa_enabled: result.mfa_enabled,
            mfa_secret: result.mfa_secret,
        })
    }

    /// Deletes a user
    pub async fn delete_user(&self, id: UserId, tenant_id: TenantId) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1 AND tenant_id = $2
            "#,
            id.0 as uuid::Uuid,
            tenant_id.0 as uuid::Uuid,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Lists all users
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let results = sqlx::query!(
            r#"
            SELECT id, tenant_id, email, password_hash, active, roles, last_login, created_at, updated_at, mfa_enabled, mfa_secret
            FROM users
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(results
            .into_iter()
            .map(|r| User {
                id: UserId(r.id),
                tenant_id: TenantId(r.tenant_id),
                email: r.email,
                password_hash: r.password_hash,
                active: r.active,
                roles: convert_roles(Some(r.roles)),
                last_login: convert_to_offset(r.last_login),
                created_at: to_offset_datetime(r.created_at),
                updated_at: to_offset_datetime(r.updated_at),
                mfa_enabled: r.mfa_enabled,
                mfa_secret: r.mfa_secret,
            })
            .collect())
    }
}

impl Default for UserRepository {
    fn default() -> Self {
        Self::new(Database::default().get_pool())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::database::tests::create_test_db;
    use crate::modules::tenant::models::Tenant;
    use std::time::Duration;

    async fn setup_test_tenant(db: &Database) -> Result<Tenant> {
        let tenant = Tenant::new(
            "Test Tenant".to_string(),
            format!("{}.example.com", Uuid::new_v4()),
        );
        let mut retries = 3;
        while retries > 0 {
            match sqlx::query!(
                r#"INSERT INTO tenants (id, name, domain, active) VALUES ($1, $2, $3, $4)"#,
                tenant.id.0 as uuid::Uuid,
                tenant.name,
                tenant.domain,
                tenant.active
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
        Ok(tenant)
    }

    #[tokio::test]
    async fn test_user_crud() {
        let (db, _container) = create_test_db().await.unwrap();
        let repository = UserRepository::new(db.get_pool());

        // Create test tenant
        let tenant = setup_test_tenant(&db).await.unwrap();

        // Create test user
        let user = User {
            id: UserId(Uuid::new_v4()),
            tenant_id: tenant.id,
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            active: true,
            roles: vec![],
            last_login: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            mfa_enabled: false,
            mfa_secret: None,
        };

        let mut retries = 3;
        let created = loop {
            match repository.create_user(user.clone()).await {
                Ok(u) => break u,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to create user: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };
        assert_eq!(created.email, user.email);

        // Test get_user_by_email
        let mut retries = 3;
        let retrieved = loop {
            match repository
                .get_user_by_email(&user.email, user.tenant_id)
                .await
            {
                Ok(Some(u)) => break u,
                Ok(None) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("User not found");
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                },
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to get user: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        };
        assert_eq!(retrieved.id, user.id);
        assert_eq!(retrieved.email, user.email);

        // Test update_last_login
        let mut retries = 3;
        while retries > 0 {
            match repository.update_last_login(user.id).await {
                Ok(_) => break,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to update last login: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
            }
        }
    }
}
