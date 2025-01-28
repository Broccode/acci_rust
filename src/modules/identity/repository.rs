use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::core::database::Database;
use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
    traits::TenantAware,
};
use super::models::{User, Role, Permission, PermissionAction};

/// Repository for user-related database operations
#[derive(Debug, Clone)]
pub struct UserRepository {
    db: Database,
}

impl UserRepository {
    /// Creates a new UserRepository instance
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Creates a new user in the database
    pub async fn create_user(&self, user: &User) -> Result<User> {
        let pool = self.db.pool();
        
        // First create the user
        let user_id = sqlx::query!(
            r#"
            INSERT INTO users (id, tenant_id, email, password_hash, active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
            user.id.0,
            user.tenant_id.0,
            user.email,
            user.password_hash,
            user.active,
            user.created_at,
            user.updated_at,
        )
        .fetch_one(pool)
        .await?
        .id;

        // Then assign roles
        for role in &user.roles {
            sqlx::query!(
                r#"
                INSERT INTO user_roles (user_id, role_id)
                VALUES ($1, $2)
                "#,
                user_id,
                role.id,
            )
            .execute(pool)
            .await?;
        }

        self.get_user_by_id(UserId(user_id)).await
    }

    /// Retrieves a user by their email within a tenant
    pub async fn get_user_by_email(&self, email: &str, tenant_id: TenantId) -> Result<User> {
        let pool = self.db.pool();
        
        let user = sqlx::query!(
            r#"
            SELECT id
            FROM users
            WHERE email = $1 AND tenant_id = $2
            "#,
            email,
            tenant_id.0,
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::NotFound(format!("User not found: {}", email)))?;

        self.get_user_by_id(UserId(user.id)).await
    }

    /// Retrieves a user by their ID
    pub async fn get_user_by_id(&self, id: UserId) -> Result<User> {
        let pool = self.db.pool();
        
        let user = sqlx::query!(
            r#"
            SELECT 
                u.*,
                (
                    SELECT array_agg(r.id)
                    FROM user_roles ur
                    JOIN roles r ON ur.role_id = r.id
                    WHERE ur.user_id = u.id
                ) as role_ids
            FROM users u
            WHERE u.id = $1
            "#,
            id.0,
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::NotFound(format!("User not found: {}", id.0)))?;

        // Fetch roles with permissions if there are any role IDs
        let roles = if let Some(role_ids) = user.role_ids {
            if !role_ids.is_empty() {
                self.get_roles_with_permissions(&role_ids[..]).await?
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        Ok(User {
            id: UserId(user.id),
            tenant_id: TenantId(user.tenant_id),
            email: user.email,
            password_hash: user.password_hash,
            roles,
            active: user.active,
            last_login: user.last_login,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    /// Updates a user's last login timestamp
    pub async fn update_last_login(&self, user_id: UserId) -> Result<()> {
        let pool = self.db.pool();
        
        sqlx::query!(
            r#"
            UPDATE users
            SET last_login = $1, updated_at = $1
            WHERE id = $2
            "#,
            OffsetDateTime::now_utc(),
            user_id.0,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Retrieves roles with their permissions
    async fn get_roles_with_permissions(&self, role_ids: &[Uuid]) -> Result<Vec<Role>> {
        let pool = self.db.pool();
        
        let roles = sqlx::query!(
            r#"
            SELECT 
                r.id,
                r.name,
                (
                    SELECT jsonb_agg(
                        jsonb_build_object(
                            'id', p.id,
                            'name', p.name,
                            'resource', p.resource,
                            'action', p.action
                        )
                    )
                    FROM role_permissions rp
                    JOIN permissions p ON rp.permission_id = p.id
                    WHERE rp.role_id = r.id
                ) as permissions
            FROM roles r
            WHERE r.id = ANY($1)
            "#,
            role_ids,
        )
        .fetch_all(pool)
        .await?;

        Ok(roles
            .into_iter()
            .map(|role| {
                let permissions = role.permissions
                    .map(|perms| {
                        serde_json::from_value(perms).unwrap_or_default()
                    })
                    .unwrap_or_default();

                Role {
                    id: role.id,
                    name: role.name,
                    permissions,
                }
            })
            .collect())
    }
}

#[async_trait::async_trait]
impl TenantAware for UserRepository {
    async fn set_tenant_context(&self, tenant_id: TenantId) -> Result<()> {
        self.db.set_tenant_context(tenant_id).await
    }

    async fn clear_tenant_context(&self) -> Result<()> {
        self.db.clear_tenant_context().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Implement tests
    // Will need to set up test database and migrations first
}