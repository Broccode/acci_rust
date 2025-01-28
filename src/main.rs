use std::env;
use tracing::{info, warn};
use tracing_subscriber::{
    prelude::*,
    EnvFilter,
    fmt,
    Registry,
};

use crate::core::{config::ServerConfig, server::Server};

mod core;
mod modules;
mod shared;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    Registry::default()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "acci_rust=debug,tower_http=debug,axum::rejection=trace".into()
        }))
        .with(fmt::layer())
        .init();

    info!("Starting ACCI Framework...");

    // Set up database URL for SQLx if not already set
    if env::var("DATABASE_URL").is_err() {
        let db_url = "postgres://localhost/acci_rust";
        env::set_var("DATABASE_URL", db_url);
        warn!(
            "DATABASE_URL not set, using default: {}",
            db_url
        );
    }

    // Load configuration
    let config = ServerConfig::default_dev();

    // Create and run server
    let server = Server::new(&config).await?;
    server.run().await?;

    Ok(())
}
