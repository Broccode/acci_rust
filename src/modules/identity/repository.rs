use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::core::database::Database;
use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
    traits::TenantAware,
};
use super::models::{User, Role, Permission};

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

    /// Retrieves a user by their ID
    pub async fn get_user_by_id(&self, id: UserId) -> Result<User> {
        let pool = self.db.pool();
        
        let user = sqlx::query!(
            r#"
            SELECT 
                u.*,
                COALESCE(array_agg(r.id) FILTER (WHERE r.id IS NOT NULL), '{}') as role_ids
            FROM users u
            LEFT JOIN user_roles ur ON u.id = ur.user_id
            LEFT JOIN roles r ON ur.role_id = r.id
            WHERE u.id = $1
            GROUP BY u.id
            "#,
            id.0,
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::NotFound(format!("User not found: {}", id.0)))?;

        // Fetch roles with permissions if there are any
        let roles = if !user.role_ids.is_empty() {
            self.get_roles_with_permissions(&user.role_ids).await?
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

    /// Retrieves roles with their permissions
    async fn get_roles_with_permissions(&self, role_ids: &[Uuid]) -> Result<Vec<Role>> {
        let pool = self.db.pool();
        
        let roles = sqlx::query!(
            r#"
            SELECT 
                r.id,
                r.name,
                COALESCE(array_agg(p.id) FILTER (WHERE p.id IS NOT NULL), '{}') as permission_ids,
                COALESCE(array_agg(p.name) FILTER (WHERE p.id IS NOT NULL), '{}') as permission_names,
                COALESCE(array_agg(p.resource) FILTER (WHERE p.id IS NOT NULL), '{}') as permission_resources,
                COALESCE(array_agg(p.action) FILTER (WHERE p.id IS NOT NULL), '{}') as permission_actions
            FROM roles r
            LEFT JOIN role_permissions rp ON r.id = rp.role_id
            LEFT JOIN permissions p ON rp.permission_id = p.id
            WHERE r.id = ANY($1)
            GROUP BY r.id, r.name
            "#,
            role_ids,
        )
        .fetch_all(pool)
        .await?;

        Ok(roles
            .into_iter()
            .map(|role| {
                let permissions = role.permission_ids
                    .into_iter()
                    .zip(role.permission_names)
                    .zip(role.permission_resources)
                    .zip(role.permission_actions)
                    .map(|(((id, name), resource), action)| Permission {
                        id,
                        name,
                        resource,
                        action: serde_json::from_str(&action).unwrap_or_default(),
                    })
                    .collect();

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