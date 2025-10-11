use super::db::Databases;
use crate::models::user::UserModel;
use crate::utils::logging::beautify_print;
use std::io::{self, Write};
use tokio::task;
use std::process::Command;
use std::time::Duration;
use std::os::unix::process::CommandExt;
use colored::*;

pub async fn start_cli(dbs: Databases) -> anyhow::Result<()> {
    let user_model = UserModel::new(dbs.users.clone());

    loop {

        print!("{}", "DUCO Server $ ".yellow());
        io::Write::flush(&mut io::stdout()).ok();

        let input = match task::spawn_blocking(|| {
            let mut s = String::new();
            io::stdin().read_line(&mut s).ok();
            s
        }).await {
            Ok(s) => s,
            Err(_) => continue,
        };

        let cmd = input.trim();
        if cmd.is_empty() { continue; }

        let parts: Vec<&str> = cmd.split_whitespace().collect();

        match parts[0] {
            "exit" => {
                beautify_print("Exiting CLI...", "success");
                break;
            }
            "help" => {
                beautify_print("
                Commands Available:
                - exit
                - help
                - restart
                - user add/info/del/update
                ", "?");
            }
            "restart" => {
                beautify_print("Are you sure you want to restart? (Y/n)", "?");

                let confirm = task::spawn_blocking(|| {
                    let mut s = String::new();
                    io::stdin().read_line(&mut s).ok();
                    s
                })
                .await
                .unwrap_or_default();

                let c = confirm.trim().to_lowercase();
                if c.is_empty() || c == "y" {
                    let _ = Command::new("pkill").arg("-9").arg("rsync").status();

                    let exe = std::env::current_exe()?;
                    let args: Vec<String> = std::env::args().collect();
                    std::process::Command::new(exe)
                        .args(&args[1..])
                        .exec();

                    beautify_print("Restarting...", "success");
                } else {
                    beautify_print("Canceled", "warning");
                }
            }
            "user" => {
                if parts.len() < 2 {
                    beautify_print("
                    Usage:
                    - user add <username> <password> <email> <balance>
                    - user info <username>
                    - user del <username>
                    - user upd <username> <field> <value>
                    ", "warning");
                    continue;
                }
                match parts[1] {
                    "add" if parts.len() == 6 => {
                        let username = parts[2];
                        let password = parts[3].to_string();
                        let email = parts[4];
                        let balance: f64 = parts[5].parse().unwrap_or(0.0);

                        let username = username.to_string();
                        let email = email.to_string();

                        let handle = task::spawn_blocking(move || {
                            (username, password, email, balance)
                        }).await?;

                        let (username, password, email, balance) = handle;
                        match user_model.add_user(&username, &password, &email, balance).await {
                            Ok(_) => beautify_print("User added", "success"),
                            Err(e) => beautify_print(&format!("Error: {:?}", e), "error"),
                        }
                    }
                    "info" if parts.len() == 3 => {
                        let username = parts[2];
                        match user_model.get_user(username).await {
                            Ok(Some(u)) => beautify_print(&format!("{:?}", u), "?"),
                            Ok(None) => beautify_print("User not found", "warning"),
                            Err(e) => beautify_print(&format!("Error: {:?}", e), "error"),
                        }
                    }
                    "del" if parts.len() == 3 => {
                        match user_model.delete_user(parts[2]).await {
                            Ok(_) => beautify_print("Deleted", "success"),
                            Err(_) => beautify_print("Delete failed", "error"),
                        }
                    }
                    "upd" if parts.len() == 5 => {
                        match user_model.update_user(parts[2], parts[3], parts[4]).await {
                            Ok(_) => beautify_print("Updated", "success"),
                            Err(_) => beautify_print("Update failed", "error"),
                        }
                    }
                    _ => beautify_print("
                        Usage:
                        - user add <username> <password> <email> <balance>
                        - user info <username>
                        - user del <username>
                        - user upd <username> <field> <value>
                    ", "warning"),
                }
            }
            other => {
                beautify_print(&format!("Unknown command: {}", other), "warning");
            }
        }
    }

    Ok(())
}
