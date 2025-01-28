//! ACCI Framework library

pub mod core;
pub mod modules;
pub mod shared;

// Re-export commonly used items
pub use crate::{
    core::{config, database, server},
    modules::{identity, tenant},
    shared::{error, types, traits},
};