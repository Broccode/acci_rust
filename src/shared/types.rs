use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a tenant identifier in the multi-tenant system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub Uuid);

impl TenantId {
    /// Creates a new TenantId with a random UUID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Represents a user identifier in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    /// Creates a new UserId with a random UUID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}