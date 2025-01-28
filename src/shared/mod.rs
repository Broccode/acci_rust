//! Shared kernel containing common types and utilities

pub mod types;
pub mod error;
pub mod traits;

// Re-export commonly used types
pub use types::{TenantId, UserId};
pub use error::Error;