use sqlx::{Pool, Postgres};
use time::OffsetDateTime;

use crate::{
    core::database::Database,
    modules::identity::models::{Role, RoleType, User},
    shared::{
        error::Result,
        types::{TenantId, UserId},
    },
};

/// Helper function to convert database roles to domain roles
fn convert_roles(roles: Option<Vec<String>>) -> Vec<Role> {
    match roles {
        Some(roles) => roles
            .into_iter()
            .map(|r| Role::new(RoleType::User, r))
            .collect(),
        None => Vec::new(),
    }
}

/// Helper function to convert Option<OffsetDateTime> to OffsetDateTime
fn convert_datetime(dt: Option<Option<OffsetDateTime>>) -> Option<OffsetDateTime> {
    dt.flatten()
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
            tenant_id.0,
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
            last_login: r.last_login,
            created_at: r.created_at,
            updated_at: r.updated_at,
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
            user_id.0,
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
            user.id.0,
            user.tenant_id.0,
            user.email,
            user.password_hash,
            user.active,
            &user.roles.iter().map(|r| r.to_string()).collect::<Vec<String>>(),
            user.created_at,
            user.updated_at,
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
            last_login: result.last_login,
            created_at: result.created_at,
            updated_at: result.updated_at,
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
            id.0,
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
            last_login: r.last_login,
            created_at: r.created_at,
            updated_at: r.updated_at,
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
            &user.roles.iter().map(|r| r.to_string()).collect::<Vec<String>>(),
            user.updated_at,
            user.mfa_enabled,
            user.mfa_secret,
            user.id.0,
            user.tenant_id.0,
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
            last_login: result.last_login,
            created_at: result.created_at,
            updated_at: result.updated_at,
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
            id.0,
            tenant_id.0,
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
                last_login: r.last_login,
                created_at: r.created_at,
                updated_at: r.updated_at,
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

    #[tokio::test]
    async fn test_user_crud() {
        let (db, _container) = create_test_db().await.unwrap();
        let repo = UserRepository::new(db.get_pool());

        // Create test tenant
        let tenant_id = TenantId(Uuid::new_v4());
        sqlx::query!(
            "INSERT INTO tenants (id, name) VALUES ($1, $2)",
            tenant_id.0,
            "Test Tenant"
        )
        .execute(&repo.pool)
        .await
        .unwrap();

        // Create test user
        let user = User {
            id: UserId(Uuid::new_v4()),
            tenant_id,
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

        let created = repo.create_user(user.clone()).await.unwrap();
        assert_eq!(created.email, user.email);

        // Test get_user_by_email
        let retrieved = repo
            .get_user_by_email(&user.email, user.tenant_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.id, user.id);
        assert_eq!(retrieved.email, user.email);

        // Test update_last_login
        repo.update_last_login(user.id).await.unwrap();
    }
}
