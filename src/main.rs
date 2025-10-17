mod core;
mod utils;
mod models;
mod network;

use crate::network::tcp_server;
use core::db::Databases;
use core::config::AppConfig;
use core::cli;
use std::process;
use tracing::info;
use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::core::app::App;

/// Main entry point for the DUCO Server application.
///
/// This function initializes the database connections, starts the TCP server and CLI tasks,
/// waits for the CLI task to finish and then closes all database connections before exiting the process.
///
/// The function will return an error if any of the following operations fail:
/// - Loading the configuration file
/// - Initializing the database connections
/// - Starting the TCP server task
/// - Starting the CLI task
/// - Waiting for the CLI task to finish
/// - Closing any of the database connections

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = AppConfig::load(Some("config.toml")).expect("Failed to load Config.toml");
    let dbs = Databases::init(&cfg).await?;
    let app = App::new();

    let app_for_cli = Arc::clone(&app);
    let app_for_tcp = Arc::clone(&app);

    info!("Starting server on {}:{}", cfg.server.tcp_host, cfg.server.tcp_port);

    let _server_task = tokio::spawn(network::tcp_server::start_tcp_server(dbs.clone(), cfg.clone(), app_for_tcp));
    let cli_task = tokio::spawn(core::cli::start_cli(dbs.clone(), app_for_cli));

    match cli_task.await {
        Ok(Ok(_)) => info!("CLI finished, terminating process"),
        Ok(Err(e)) => tracing::error!("CLI returned error: {:?}", e),
        Err(e) => tracing::error!("CLI panicked: {:?}", e),
    }

    for (name, db) in [("users", &dbs.users), ("transactions", &dbs.transactions), ("stats", &dbs.stats)] {
        db.close().await;
        println!("✅ Closed DB connection for {}", name);
    }

    process::exit(0);

    Ok(())
}
