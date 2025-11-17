use crate::core::app::App;
use crate::core::app::Connection;
use crate::core::config::AppConfig;
use crate::core::db::Databases;
use crate::models::transaction::TransactionModel;
use crate::models::user::UserModel;
use crate::utils::crypto::verify_and_upgrade;
use crate::utils::helpers::block_ip_background;
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::{Duration, timeout};

/// Starts a TCP server that listens for incoming connections and
/// handles them asynchronously.
pub async fn start_tcp_server(dbs: Databases, cfg: AppConfig, app: Arc<App>) -> Result<()> {
    let addr = format!("{}:{}", cfg.server.tcp_host, cfg.server.tcp_port);
    let listener = TcpListener::bind(&addr).await?;

    app.log(format!("TCP Server Listening on {}", addr));

    let user_model = Arc::new(UserModel::new(dbs.users.clone()));
    let tx_model = Arc::new(TransactionModel::new(dbs.transactions.clone()));

    loop {
        let (socket, peer) = listener.accept().await?;
        let ip = peer.ip().to_string();
        let port = peer.port();

        {
            app.add_connection(Connection {
                name: format!("Client-{}", port),
                ip: ip.clone(),
                port,
                status: true,
                data_sent: 0,
                data_received: 0,
                uptime: chrono::Local::now(),
            });
            app.log(format!("New connection from {}:{}", ip, port));
        }

        let user_model = Arc::clone(&user_model);
        let tx_model = Arc::clone(&tx_model);
        let app = Arc::clone(&app);

        tokio::spawn(async move {
            let res = handle_client(socket, user_model, tx_model, &app).await;

            match res {
                Ok(_) => {
                    app.log(format!("Client disconnected: {}:{}", ip, port));
                }
                Err(e) => {
                    app.log(format!("Error handling client {}:{} — {:?}", ip, port, e));
                }
            }
            app.remove_connection(&ip, port);
        });
    }
}

/// Handles a single client connection.
async fn handle_client(
    mut socket: tokio::net::TcpStream,
    user_model: Arc<UserModel>,
    tx_model: Arc<TransactionModel>,
    app: &Arc<App>,
) -> Result<()> {
    let mut buf = [0u8; 2048];
    let mut logged_in = false;
    let mut username: Option<String> = None;
    let mut bytes_sent_accum: usize = 0;
    let mut bytes_recv_accum: usize = 0;
    const FLUSH_THRESHOLD: usize = 1024;
    let mut failed_logins: u32 = 0;
    const FAIL_THRESHOLD: u32 = 3;

    // If no data is received within this duration, the connection will be closed.
    let idle_timeout = Duration::from_secs(300); // 5 minutes

    loop {
        let read_result = timeout(idle_timeout, socket.read(&mut buf)).await;

        let n = match read_result {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => {
                app.log(format!("Read error: {:?}", e));
                break;
            }
            Err(e) => {
                app.log(format!("Timeout reading socket: {:?}", e));
                break;
            }
        };

        if n == 0 {
            app.log(format!(
                "TCPClient disconnected: {}",
                socket
                    .peer_addr()
                    .unwrap_or_else(|_| "unknown".parse().unwrap())
            ));
            break;
        }

        bytes_recv_accum += n;
        if bytes_recv_accum >= FLUSH_THRESHOLD {
            let addr = socket.peer_addr().unwrap();
            app.update_connection_data_received(
                &addr.ip().to_string(),
                addr.port(),
                bytes_recv_accum as u64,
            );
            bytes_recv_accum = 0;
        }

        let msg = String::from_utf8_lossy(&buf[..n]).trim().to_string();
        let parts: Vec<&str> = msg.split(',').collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0].trim().to_uppercase().as_str() {
            "VER" => {
                socket.write_all(b"MASTERSERVER 1.0\n").await?;
            }

            "LOGI" => {
                if parts.len() < 3 {
                    socket
                        .write_all(b"NO,Missing username or password\n")
                        .await?;
                    continue;
                }
                let user_input = parts[1].trim();
                let pass_input = parts[2].trim();

                match user_model.get_user(user_input).await {
                    Ok(Some(u)) => {
                        if let Some(new_hash) =
                            verify_and_upgrade(pass_input, &u.username, &u.password, &app)?
                        {
                            if new_hash != u.password {
                                user_model.update_password(&u.username, &new_hash).await?;
                            }
                            logged_in = true;
                            username = Some(u.username.clone());
                            socket.write_all(b"OK,Authenticated\n").await?;
                            app.log(format!("User '{}' logged in successfully", user_input));
                            failed_logins = 0;
                        } else {
                            let ip = socket.peer_addr().unwrap().ip().to_string();
                            app.log(format!("Auth verify error for {}: {:?}", user_input, ip));
                            socket.write_all(b"NO,Invalid password\n").await?;
                            failed_logins += 1;
                        }
                    }
                    Ok(_none) => {
                        socket.write_all(b"NO,User not found\n").await?;
                        app.log(format!("Failed login attempt: {}", user_input));
                        failed_logins += 1;
                    }
                    Err(e) => {
                        socket
                            .write_all(format!("NO,Error: {:?}\n", e).as_bytes())
                            .await?;
                        app.log(format!("Login error for {}: {:?}", user_input, e));
                        failed_logins += 1;
                    }
                }
            }

            "BALA" => {
                if !logged_in {
                    socket.write_all(b"NO,Not logged in\n").await?;
                    continue;
                }
                if let Some(ref user) = username {
                    match user_model.get_user(user).await {
                        Ok(Some(u)) => {
                            socket
                                .write_all(format!("{:.6}\n", u.balance).as_bytes())
                                .await?;
                        }
                        Ok(_none) => socket.write_all(b"NO,User not found\n").await?,
                        Err(e) => {
                            socket
                                .write_all(format!("NO,Error: {:?}\n", e).as_bytes())
                                .await?;
                        }
                    }
                }
            }

            "GTXL" => {
                if !logged_in {
                    socket.write_all(b"NO,Not logged in\n").await?;
                    continue;
                }

                let target_user = parts.get(1).map(|s| s.trim()).unwrap_or("");
                if target_user.is_empty() {
                    socket.write_all(b"NO,Missing username\n").await?;
                    continue;
                }

                let rows = match tx_model.get_by_username(target_user).await {
                    Ok(r) => r,
                    Err(e) => {
                        app.log(format!("Error fetching transactions: {:?}", e));
                        socket
                            .write_all(b"NO,Error fetching transactions\n")
                            .await?;
                        continue;
                    }
                };

                let data_str =
                    serde_json::to_string(&rows).unwrap_or_else(|_| "{}".to_string()) + "\n";

                if let Err(e) = socket.write_all(data_str.as_bytes()).await {
                    app.log(format!("Failed to send GTXL data: {:?}", e));
                }
            }

            "PING" => {
                socket.write_all(b"PONG\n").await?;
            }

            "EXIT" => {
                socket.write_all(b"Bye!\n").await?;
                break;
            }

            _ => {
                socket.write_all(b"NO,Unknown command\n").await?;
            }
        }

        if failed_logins >= FAIL_THRESHOLD {
            if let Ok(peer_addr) = socket.peer_addr() {
                let ip_to_block = peer_addr.ip();
                let app_clone = Arc::clone(app);
                
                tokio::spawn(async move {
                    if let Err(e) = block_ip_background(app_clone, ip_to_block, Some(Duration::from_hours(1))).await {
                        eprintln!("Failed to block IP {}: {:?}", ip_to_block, e);
                    }
                });

                let _ = socket
                    .write_all(b"NO,Too many failed attempts. Connection closed.\n")
                    .await;
            }

            break;
        }

        if bytes_sent_accum >= FLUSH_THRESHOLD {
            let addr = socket.peer_addr().unwrap();
            app.update_connection_data_sent(
                &addr.ip().to_string(),
                addr.port(),
                bytes_sent_accum as u64,
            );
            bytes_sent_accum = 0;
        }
    }

    if bytes_sent_accum > 0 || bytes_recv_accum > 0 {
        let addr = socket.peer_addr().unwrap();
        if bytes_sent_accum > 0 {
            app.update_connection_data_sent(
                &addr.ip().to_string(),
                addr.port(),
                bytes_sent_accum as u64,
            );
        }
        if bytes_recv_accum > 0 {
            app.update_connection_data_received(
                &addr.ip().to_string(),
                addr.port(),
                bytes_recv_accum as u64,
            );
        }
    }

    Ok(())
}
