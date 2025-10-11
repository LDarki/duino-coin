use crate::core::db::Databases;
use crate::core::config::AppConfig;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::Result;
use crate::models::transaction::TransactionModel;
use crate::models::user::UserModel;
use crate::utils::logging::beautify_print;
use crate::utils::crypto::verify_and_upgrade;
use std::collections::BTreeMap;
use serde_json::json;

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
    beautify_print(&format!("✅ Server listening on {}\n", addr), "success");

    loop {
        let (mut socket, peer) = listener.accept().await?;
        beautify_print(&format!("TCPClient connected: {}", peer), "info");
        tokio::spawn(handle_client(socket, dbs.clone()));
    }
}

/// Handles a single client connection.
async fn handle_client(mut socket: tokio::net::TcpStream, dbs: Databases) -> Result<()> {
    let user_model = UserModel::new(dbs.users.clone());
    let tx_model = TransactionModel::new(dbs.transactions.clone());

    let mut buf = [0u8; 2024];
    let mut logged_in = false;
    let mut username: Option<String> = None;

    loop {
        let n = match socket.read(&mut buf).await {
            Ok(n) => n,
            Err(e) => {
                beautify_print(
                    &format!("Error reading from {}: {:?}", socket.peer_addr().unwrap_or_else(|_| "unknown".parse().unwrap()), e),
                    "error",
                );
                break;
            }
        };

        if n == 0 {
            beautify_print(
                &format!("TCPClient disconnected: {}", socket.peer_addr()?),
                "info",
            );
            break;
        }

        let msg = String::from_utf8_lossy(&buf[..n]).trim().to_string();
        let parts: Vec<&str> = msg.split(',').collect();
        if parts.is_empty() { continue; }

        match parts[0].trim().to_uppercase().as_str() {

            "VER" => {
                socket.write_all(b"MASTERSERVER 1.0\n").await?;
            }

            "LOGI" => {
                if parts.len() < 3 {
                    socket.write_all(b"NO,Missing username or password\n").await?;
                    return Ok(());
                }
                let user_input = parts[1].trim();
                let pass_input = parts[2].trim();

                match user_model.get_user(user_input).await {
                    Ok(Some(u)) => {
                        if let Some(new_hash) = verify_and_upgrade(pass_input, &u.username, &u.password)? {
                            if new_hash != u.password {
                                user_model.update_password(&u.username, &new_hash).await?;
                            }
                            logged_in = true;
                            username = Some(u.username.clone());
                            socket.write_all(b"OK,Authenticated\n").await?;
                            beautify_print(
                                &format!("User '{}' logged in successfully", user_input),
                                "success",
                            );
                        } else {
                            socket.write_all(b"NO,Invalid password\n").await?;
                        }
                    }
                    Ok(None) => socket.write_all(b"NO,User not found\n").await?,
                    Err(e) => socket.write_all(format!("NO,Error: {:?}\n", e).as_bytes()).await?,
                }
            }

            "BALA" => {
                if !logged_in {
                    socket.write_all(b"NO,Not logged in\n").await?;
                    return Ok(());
                }
                if let Some(ref user) = username {
                    match user_model.get_user(user).await {
                        Ok(Some(u)) => socket.write_all(format!("{:.6}\n", u.balance).as_bytes()).await?,
                        Ok(None) => socket.write_all(b"NO,User not found\n").await?,
                        Err(e) => socket.write_all(format!("NO,Error: {:?}\n", e).as_bytes()).await?,
                    }
                }
            }

            "GTXL" => {
                if !logged_in {
                    socket.write_all(b"NO,Not logged in\n").await?;
                    return Ok(());
                }

                let target_user = parts.get(1).map(|s| s.trim()).unwrap_or("");
                if target_user.is_empty() {
                    socket.write_all(b"NO,Missing username\n").await?;
                    return Ok(());
                }

                let count = parts.get(2)
                    .and_then(|s| s.trim().parse::<usize>().ok())
                    .unwrap_or(15)
                    .min(15);

                let rows = match tx_model.get_all().await {
                    Ok(r) => r,
                    Err(e) => {
                        beautify_print(&format!("Error fetching transactions: {:?}", e), "error");
                        socket.write_all(format!("NO,Error fetching transactions\n").as_bytes()).await?;
                        return Ok(());
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
                        if idx >= count { break; }
                    }
                }

                let data_str = serde_json::to_string(&filtered).unwrap_or_else(|_| "{}".to_string()) + "\n";

                if let Err(e) = socket.write_all(data_str.as_bytes()).await {
                    beautify_print(&format!("Failed to send GTXL data: {:?}", e), "error");
                    return Ok(());
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
    }
    Ok(())
}