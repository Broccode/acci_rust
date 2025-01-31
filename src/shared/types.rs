use serde::{Deserialize, Serialize};
use sqlx::postgres::PgArgumentBuffer;
use uuid::Uuid;

/// Tenant ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub Uuid);

/// User ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl TenantId {
    /// Creates a new TenantId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl UserId {
    /// Creates a new UserId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl From<Uuid> for TenantId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<Uuid> for UserId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<TenantId> for Uuid {
    fn from(id: TenantId) -> Self {
        id.0
    }
}

impl From<UserId> for Uuid {
    fn from(id: UserId) -> Self {
        id.0
    }
}

impl sqlx::Type<sqlx::Postgres> for TenantId {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <Uuid as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for TenantId {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> sqlx::encode::IsNull {
        <Uuid as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.0, buf)
    }
}

impl sqlx::Type<sqlx::Postgres> for UserId {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <Uuid as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for UserId {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> sqlx::encode::IsNull {
        <Uuid as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.0, buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_id_creation() {
        let id = TenantId::new();
        assert_ne!(id.0, Uuid::nil());
    }

    #[test]
    fn test_user_id_creation() {
        let id = UserId::new();
        assert_ne!(id.0, Uuid::nil());
    }

    #[test]
    fn test_tenant_id_conversion() {
        let uuid = Uuid::new_v4();
        let tenant_id = TenantId::from(uuid);
        assert_eq!(tenant_id.0, uuid);
        assert_eq!(Uuid::from(tenant_id), uuid);
    }

    #[test]
    fn test_user_id_conversion() {
        let uuid = Uuid::new_v4();
        let user_id = UserId::from(uuid);
        assert_eq!(user_id.0, uuid);
        assert_eq!(Uuid::from(user_id), uuid);
    }
}
