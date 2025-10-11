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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = AppConfig::load(Some("config.toml")).expect("Failed to load Config.toml");
    let dbs = Databases::init(&cfg).await?;

    info!("Starting server on {}:{}", cfg.server.tcp_host, cfg.server.tcp_port);

    let _server_task = tokio::spawn(network::tcp_server::start_tcp_server(dbs.clone(), cfg.clone()));
    let cli_task = tokio::spawn(core::cli::start_cli(dbs.clone()));

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
