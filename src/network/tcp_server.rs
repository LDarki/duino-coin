use crate::core::db::Databases;
use crate::core::config::AppConfig;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::Result;

/// Starts a TCP server that listens for incoming connections and
/// handles them asynchronously.
///
/// This function binds a TCP listener to the specified address and
/// port, and then enters a loop where it accepts incoming connections
/// and spawns a new task to handle each connection using the
/// `handle_client` function.
///
/// The function will return an error if binding the TCP listener fails.
pub async fn start_tcp_server(dbs: Databases, cfg: AppConfig) -> Result<()> {
    let addr = format!("{}:{}", cfg.server.tcp_host, cfg.server.tcp_port);
    let listener = TcpListener::bind(&addr).await?;
    println!("✅ Server listening on {}\n", addr);

    loop {
        let (mut socket, peer) = listener.accept().await?;
        println!("Client connected: {}", peer);
        tokio::spawn(handle_client(socket, dbs.clone()));
    }
}

/// Handles a single client connection.
async fn handle_client(mut socket: tokio::net::TcpStream, dbs: Databases) -> Result<()> {
    let mut buf = [0u8; 2024];
    loop {

        let n = socket.read(&mut buf).await?;

        if n == 0 {
            break;
        }

        let msg = String::from_utf8_lossy(&buf[..n]);
        println!("Received: {}", msg);
        socket.write_all(b"Message received\n").await?;
    }
    Ok(())
}