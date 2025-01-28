//! Core framework functionality

pub mod config;
pub mod database;
pub mod server;

// Re-export commonly used items
pub use config::Config;
pub use database::Database;
pub use server::Server;

/// Represents the core ACCI Framework
pub struct ACCIFramework {
    config: Config,
    database: Database,
    server: Server,
}

impl ACCIFramework {
    /// Creates a new instance of the ACCI Framework
    pub async fn new(config: Config) -> crate::shared::error::Result<Self> {
        let database = Database::connect(&config.database).await?;
        let server = Server::new(&config.server);

        Ok(Self {
            config,
            database,
            server,
        })
    }

    /// Starts the framework
    pub async fn run(self) -> crate::shared::error::Result<()> {
        // Initialize and start all components
        self.server.run().await
    }
}