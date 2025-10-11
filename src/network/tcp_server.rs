use crate::core::db::Databases;
use crate::core::config::AppConfig;
use crate::core::app::App;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::Result;
use crate::models::transaction::TransactionModel;
use crate::models::user::UserModel;
use crate::utils::logging::beautify_print;
use crate::utils::crypto::verify_and_upgrade;
use std::collections::BTreeMap;
use serde_json::json;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::utils::logging::Tab;
use chrono::{DateTime, Local, Duration};
use crate::core::app::Connection;

/// Starts a TCP server that listens for incoming connections and
/// handles them asynchronously.
pub async fn start_tcp_server(
    dbs: Databases,
    cfg: AppConfig,
    app: Arc<Mutex<App>>,
) -> Result<()> {
    let addr = format!("{}:{}", cfg.server.tcp_host, cfg.server.tcp_port);
    let listener = TcpListener::bind(&addr).await?;
    beautify_print(&format!("✅ Server listening on {}\n", addr), "success", Some(app.clone()), Tab::Logs);

    let user_model = Arc::new(UserModel::new(dbs.users.clone()));
    let tx_model = Arc::new(TransactionModel::new(dbs.transactions.clone()));

    loop {
        let (socket, peer) = listener.accept().await?;
        let ip = peer.ip().to_string();
        let port = peer.port();

        {
            let mut app = app.lock().await;
            app.add_connection(Connection {
                name: format!("Client-{}", port),
                ip: ip.clone(),
                port,
                status: true,
                data_sent: 0,
                data_received: 0,
                uptime: chrono::Local::now(),
            });
            app.log(format!("🟢 New connection from {}:{}", ip, port));
        }

        let user_model = Arc::clone(&user_model);
        let tx_model = Arc::clone(&tx_model);
        let app = Arc::clone(&app);

        tokio::spawn(async move {
            let res = handle_client(socket, user_model, tx_model, app.clone()).await;

            let mut app = app.lock().await;
            match res {
                Ok(_) => {
                    app.log(format!("🔴 Client disconnected: {}:{}", ip, port));
                }
                Err(e) => {
                    app.log(format!("❌ Error handling client {}:{} — {:?}", ip, port, e));
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
    app: Arc<Mutex<App>>,
) -> Result<()> {
    let mut buf = [0u8; 2048];
    let mut logged_in = false;
    let mut username: Option<String> = None;
    let mut bytes_sent_accum: usize = 0;
    let mut bytes_recv_accum: usize = 0;
    const FLUSH_THRESHOLD: usize = 256;

    loop {
        let n = match socket.read(&mut buf).await {
            Ok(n) => n,
            Err(e) => {
                let mut app = app.lock().await;
                app.log(format!("Error reading from socket: {:?}", e));
                break;
            }
        };

        if n == 0 {
            let mut app = app.lock().await;
            app.log(format!(
                "TCPClient disconnected: {}",
                socket.peer_addr().unwrap_or_else(|_| "unknown".parse().unwrap())
            ));
            break;
        }

        bytes_recv_accum += n;
        if bytes_recv_accum >= FLUSH_THRESHOLD {
            let addr = socket.peer_addr().unwrap();
            let mut app = app.lock().await;
            app.update_connection_data_received(&addr.ip().to_string(), addr.port(), bytes_recv_accum);
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
                    socket.write_all(b"NO,Missing username or password\n").await?;
                    continue;
                }
                let user_input = parts[1].trim();
                let pass_input = parts[2].trim();

                match user_model.get_user(user_input).await {
                    Ok(Some(u)) => {
                        if let Some(new_hash) = verify_and_upgrade(pass_input, &u.username, &u.password, Some(app.clone()))? {
                            if new_hash != u.password {
                                user_model.update_password(&u.username, &new_hash).await?;
                            }
                            logged_in = true;
                            username = Some(u.username.clone());
                            socket.write_all(b"OK,Authenticated\n").await?;
                            let mut app = app.lock().await;
                            app.log(format!("User '{}' logged in successfully", user_input));
                        } else {
                            socket.write_all(b"NO,Invalid password\n").await?;
                        }
                    }
                    Ok(None) => {
                        socket.write_all(b"NO,User not found\n").await?;
                        let mut app = app.lock().await;
                        app.log(format!("Failed login attempt: {}", user_input));
                    }
                    Err(e) => {
                        socket.write_all(format!("NO,Error: {:?}\n", e).as_bytes()).await?;
                        let mut app = app.lock().await;
                        app.log(format!("Login error for {}: {:?}", user_input, e));
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
                        Ok(None) => socket.write_all(b"NO,User not found\n").await?,
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

                let count = parts
                    .get(2)
                    .and_then(|s| s.trim().parse::<usize>().ok())
                    .unwrap_or(15)
                    .min(15);

                let rows = match tx_model.get_all().await {
                    Ok(r) => r,
                    Err(e) => {
                        let mut arc_app = app.lock().await;
                        arc_app.log(format!("Error fetching transactions: {:?}", e));
                        socket.write_all(b"NO,Error fetching transactions\n").await?;
                        continue;
                    }
                };

                let mut filtered = BTreeMap::new();
                let mut idx = 0;

                for row in rows.into_iter().rev() {
                    if row.username == target_user || row.recipient == target_user {
                        let parts: Vec<&str> = row.timestamp.split_whitespace().collect();
                        let date = parts.get(0).unwrap_or(&"");
                        let time = parts.get(1).unwrap_or(&"");

                        filtered.insert(
                            idx.to_string(),
                            json!({
                                "Date": date,
                                "Time": time,
                                "Sender": row.username,
                                "Recipient": row.recipient,
                                "Amount": row.amount,
                                "Hash": row.hash,
                                "Memo": row.memo.replace(|c: char| !c.is_ascii_alphanumeric() && !" .-:!#_+-".contains(c), " ")
                            }),
                        );
                        idx += 1;
                        if idx >= count {
                            break;
                        }
                    }
                }

                let data_str =
                    serde_json::to_string(&filtered).unwrap_or_else(|_| "{}".to_string())
                        + "\n";

                if let Err(e) = socket.write_all(data_str.as_bytes()).await {
                    let mut app = app.lock().await;
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

        if bytes_sent_accum >= FLUSH_THRESHOLD {
            let addr = socket.peer_addr().unwrap();
            let mut app = app.lock().await;
            app.update_connection_data_sent(&addr.ip().to_string(), addr.port(), bytes_sent_accum);
            bytes_sent_accum = 0;
        }
    }

    if bytes_sent_accum > 0 || bytes_recv_accum > 0 {
        let addr = socket.peer_addr().unwrap();
        let mut app = app.lock().await;
        if bytes_sent_accum > 0 {
            app.update_connection_data_sent(&addr.ip().to_string(), addr.port(), bytes_sent_accum);
        }
        if bytes_recv_accum > 0 {
            app.update_connection_data_received(&addr.ip().to_string(), addr.port(), bytes_recv_accum);
        }
    }

    Ok(())
}