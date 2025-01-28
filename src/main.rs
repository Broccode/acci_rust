use tracing::info;

use crate::core::{config::ServerConfig, server::Server};

mod core;
mod modules;
mod shared;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "acci_rust=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting ACCI Framework...");

    // Load configuration
    let config = ServerConfig::default_dev();

    // Create and run server
    let server = Server::new(&config).await?;
    server.run().await?;

    Ok(())
}
