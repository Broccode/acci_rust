use time::OffsetDateTime;
use uuid::Uuid;

use super::types::{TenantId, UserId};

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub email: String,
    pub password_hash: String,
    pub active: bool,
    pub roles: Vec<String>,
    pub last_login: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub mfa_enabled: bool,
    pub mfa_secret: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Tenant {
    pub id: TenantId,
    pub name: String,
    pub domain: Option<String>,
    pub active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct MfaBackupCode {
    pub id: Uuid,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub code: String,
    pub used: bool,
    pub created_at: OffsetDateTime,
    pub used_at: Option<OffsetDateTime>,
}
