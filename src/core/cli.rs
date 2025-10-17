use std::io::{self, Stdout};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    prelude::*,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Row, Sparkline, Table, Wrap},
    Terminal,
};

use std::env;
use sysinfo::System;

use crate::{
    core::app::App, core::app::Connection, core::app::ConnectionEvent, core::db::Databases,
    models::user::User, models::user::UserModel, utils::crypto::hash_password, utils::helpers::*,
};
use tokio::time::sleep;

use std::collections::HashMap;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

struct CliState {
    connections: HashMap<String, Connection>,
    logs: Vec<String>,
    console: Vec<String>,
    cpu_history: Vec<f32>,
    mem_history: Vec<f32>,
}

pub async fn start_cli(dbs: Databases, app: Arc<App>) -> anyhow::Result<()> {
    let user_model = Arc::new(UserModel::new(dbs.users.clone()));

    let mut log_rx = app.log_tx.subscribe();
    let mut console_rx = app.console_tx.subscribe();
    let mut conn_rx = app.conn_tx.subscribe();
    let mut metrics_rx = app.metrics_tx.subscribe();

    let mut state = CliState {
        connections: HashMap::new(),
        logs: Vec::new(),
        console: Vec::new(),
        cpu_history: Vec::new(),
        mem_history: Vec::new(),
    };

    {
        app.console(
            colored::Colorize::yellow("Type 'help' for commands, 'exit' to quit.").to_string(),
        );
    }

    let metrics_tx_clone = app.metrics_tx.clone();

    tokio::spawn(async move {
        let mut sys = System::new_all();

        loop {
            sys.refresh_cpu_all();
            sys.refresh_memory();

            let cpu_usage =
                sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32;
            let mem_used = sys.used_memory() as f32;
            let mem_total = sys.total_memory() as f32;
            let mem_percent = if mem_total > 0.0 {
                (mem_used / mem_total) * 100.0
            } else {
                0.0
            };

            let _ = metrics_tx_clone.send((cpu_usage, mem_percent));

            tokio::time::sleep(Duration::from_millis(1000)).await; // cada 1s
        }
    });

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut restart_confirmation_pending = false;

    loop {
        while let Ok(msg) = log_rx.try_recv() {
            state.logs.push(msg.to_string());
            if state.logs.len() > 100 {
                state.logs.remove(0);
            }
        }

        while let Ok(msg) = console_rx.try_recv() {
            state.console.push(msg.to_string());
            if state.console.len() > 100 {
                state.console.remove(0);
            }
        }

        while let Ok((cpu, mem)) = metrics_rx.try_recv() {
            state.cpu_history.push(cpu);
            state.mem_history.push(mem);
            if state.cpu_history.len() > 30 {
                state.cpu_history.remove(0);
            }
            if state.mem_history.len() > 30 {
                state.mem_history.remove(0);
            }
        }

        while let Ok(event) = conn_rx.try_recv() {
            match event {
                ConnectionEvent::Add(conn) => {
                    state.connections.insert(conn.ip.clone(), conn);
                }
                ConnectionEvent::Remove(ip, _port) => {
                    state.connections.remove(&ip);
                }
                ConnectionEvent::UpdateDataSent(ip, _port, bytes) => {
                    if let Some(conn) = state.connections.get_mut(&ip) {
                        conn.data_sent += bytes;
                    }
                }
                ConnectionEvent::UpdateDataReceived(ip, _port, bytes) => {
                    if let Some(conn) = state.connections.get_mut(&ip) {
                        conn.data_received += bytes;
                    }
                }
            }
        }

        let app_input = {
            let input = app.input.lock().await;
            input.clone()
        };

        draw_tui(
            &mut terminal,
            &app_input,
            &state.connections.values().cloned().collect(),
            &state.logs.join("\n"),
            &state.console.join("\n"),
            &state.cpu_history,
            &state.mem_history,
        )?;

        sleep(Duration::from_millis(100)).await;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                let mut should_break = false;

                match key.code {
                    KeyCode::Char(c) => {
                        if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
                            app.console(
                                colored::Colorize::yellow("Received Ctrl+C. Exiting...")
                                    .to_string(),
                            );
                            should_break = true;
                        } else {
                            let mut input = app.input.lock().await;
                            input.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        let mut input = app.input.lock().await;
                        input.pop();
                    }
                    KeyCode::Enter => {
                        let cmd = {
                            let mut app_input = app.input.lock().await;
                            let cmd = app_input.trim().to_string();
                            app_input.clear();
                            cmd
                        };

                        if cmd.is_empty() {
                            continue;
                        }

                        if restart_confirmation_pending {
                            if cmd.to_lowercase() == "y" {
                                app.console(colored::Colorize::green("Restarting...").to_string());
                                let exe =
                                    env::current_exe().expect("Failed to get current exe path");

                                disable_raw_mode()?;

                                terminal.clear()?;

                                let _ = Command::new(exe).args(env::args().skip(1)).exec();

                                eprintln!("Failed to exec the new process");
                                std::process::exit(1);
                            } else {
                                app.console(
                                    colored::Colorize::yellow("Restart cancelled.").to_string(),
                                );
                            }
                            restart_confirmation_pending = false;
                            continue;
                        }

                        {
                            app.console(format!("> {}", cmd));
                        }

                        match cmd.as_str() {
                            "exit" => {
                                app.log(colored::Colorize::red("Exiting CLI loop.").to_string());
                                should_break = true;
                            }
                            "clear" => {
                                state.console.clear();
                            }
                            "restart" => {
                                app.console("Are you sure you want to restart? (Y/n)".to_string());
                                restart_confirmation_pending = true;
                            }
                            _ => {
                                handle_command(cmd, &app, Arc::clone(&user_model)).await;
                            }
                        }
                    }
                    _ => {}
                }

                if should_break {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    terminal.clear()?;
    Ok(())
}

async fn handle_command(cmd: String, app: &Arc<App>, user_model: Arc<UserModel>) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    match parts.get(0).map(|s| *s) {
        Some("help") => {
            let help_msg = "Commands Available:
            - exit: Quits the TUI
            - help: Shows this message
            - restart: Re-initializes server state (needs 'Y' confirmation)
            - clear
            - user add/info/del/upd/pass";
            app.console(help_msg.to_string());
        }
        Some("user") => {
            handle_user_command(parts, app, user_model).await;
        }
        _ => {
            app.console("Unknown command. Type 'help' for a list of commands.".to_string());
        }
    }
}

async fn handle_user_command(parts: Vec<&str>, app: &Arc<App>, user_model: Arc<UserModel>) {
    let user_model_clone = Arc::clone(&user_model);

    match parts.get(1).map(|s| *s) {
        Some("add") if parts.len() == 6 => {
            let username = parts[2].to_string();
            let password = parts[3].to_string();
            let email = parts[4].to_string();
            let balance: f64 = parts[5].parse().unwrap_or(0.0);

            match user_model_clone
                .add_user(&username, &password, &email, balance)
                .await
            {
                Ok(_) => {
                    app.console(format!("User added: {}", username).green().to_string());
                }
                Err(e) => {
                    app.console(format!("Error: {}", e).red().to_string());
                }
            }
        }
        Some("info") if parts.len() == 3 => {
            let username = parts[2];
            match user_model_clone.get_user(username).await {
                Ok(Some(u)) => {
                    app.console(user_info_table(&u));
                }
                Ok(_none) => {
                    app.console(
                        format!("User '{}' not found", username)
                            .yellow()
                            .to_string(),
                    );
                }
                Err(e) => {
                    app.console(format!("Error: {:?}", e).red().to_string());
                }
            }
        }
        Some("del") if parts.len() == 3 => {
            let username = parts[2];
            match user_model_clone.delete_user(username).await {
                Ok(_) => {
                    app.console(format!("User deleted: {}", username).green().to_string());
                }
                Err(e) => {
                    app.console(format!("Delete failed: {}", e).red().to_string());
                }
            }
        }
        Some("upd") if parts.len() == 5 => {
            let username = parts[2];
            let field = parts[3];
            let value = parts[4];
            match user_model_clone.update_user(username, field, value).await {
                Ok(_) => {
                    app.console(
                        format!("User {} updated field {} to {}", username, field, value)
                            .green()
                            .to_string(),
                    );
                }
                Err(e) => {
                    app.console(format!("Update failed: {}", e).red().to_string());
                }
            }
        }
        Some("pass") if parts.len() == 4 => {
            let username = parts[2];
            let new_password = parts[3];

            let hashed_password = hash_password(&new_password).unwrap();

            match user_model_clone
                .update_password(username, &hashed_password)
                .await
            {
                Ok(_) => {
                    app.console(
                        format!("User {} password updated", username)
                            .green()
                            .to_string(),
                    );
                }
                Err(e) => {
                    app.console(format!("Update failed: {}", e).red().to_string());
                }
            }
        }
        _ => {
            let usage = "
                Usage:
                - user add <username> <password> <email> <balance>
                - user info <username>
                - user del <username>
                - user upd <username> <field> <value>
                - user pass <username> <new_password>
                ";
            app.console(usage.to_string());
        }
    }
}

pub fn draw_tui(
    terminal: &mut Terminal<CrosstermBackend<&mut Stdout>>,
    input_text: &str,
    connections: &Vec<Connection>,
    logs_text: &str,
    console_text: &str,
    cpu_history: &Vec<f32>,
    mem_history: &Vec<f32>,
) -> Result<()> {
    terminal.draw(|f| {
        let size = f.area();
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(size);

        let top_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(rows[0]);

        let bottom_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(rows[1]);

        let console_area = top_row[0];
        let console_height = console_area.height.saturating_sub(2);
        let console_line_count = console_text.lines().count() as u16;
        let console_scroll = if console_line_count > console_height {
            console_line_count - console_height
        } else {
            0
        };

        let console_combined = format!("{}\n> {}", console_text, input_text);

        let console_block = Paragraph::new(console_combined)
            .block(
                Block::default()
                    .title("DUCO Server $")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true })
            .scroll((console_scroll as u16, 0));

        let logs_area = top_row[1];
        let logs_height = logs_area.height.saturating_sub(2);
        let logs_line_count = logs_text.lines().count() as u16;
        let logs_scroll = if logs_line_count > logs_height {
            logs_line_count - logs_height
        } else {
            0
        };

        let logs_block = Paragraph::new(logs_text)
            .block(Block::default().title("Logs").borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .scroll((logs_scroll as u16, 0));

        let table_rows: Vec<Row> = connections
            .iter()
            .map(|c| {
                let status_color = if c.status { Color::Green } else { Color::Red };
                Row::new(vec![
                    c.name.clone(),
                    c.ip.clone(),
                    c.port.to_string(),
                    c.status.to_string(),
                    humanize_bytes(c.data_sent),
                    humanize_bytes(c.data_received),
                    humanize_time_since(c.uptime),
                ])
                .style(Style::default().fg(status_color))
            })
            .collect();

        let column_widths = [
            Constraint::Length(12),
            Constraint::Length(15),
            Constraint::Length(6),
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(10),
        ];

        let connections_table = Table::new(table_rows, column_widths)
            .header(Row::new(vec![
                "Name", "IP", "Port", "Status", "Sent", "Received", "Uptime",
            ]))
            .block(Block::default().title("Connections").borders(Borders::ALL));

        let cpu_last = *cpu_history.last().unwrap_or(&0.0);
        let cpu_sparkline = Sparkline::default()
            .block(
                Block::default()
                    .title(format!("CPU Usage: {:.1}%", cpu_last))
                    .borders(Borders::ALL),
            )
            .data(&cpu_history.iter().map(|v| *v as u64).collect::<Vec<u64>>())
            .style(Style::default().fg(color_for_percentage(cpu_last)))
            .max(100);

        let mem_last = *mem_history.last().unwrap_or(&0.0);
        let mem_sparkline = Sparkline::default()
            .block(
                Block::default()
                    .title(format!("Memory Usage: {:.1}%", mem_last))
                    .borders(Borders::ALL),
            )
            .data(&mem_history.iter().map(|v| *v as u64).collect::<Vec<u64>>())
            .style(Style::default().fg(color_for_percentage(mem_last)))
            .max(100);

        let metrics_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(bottom_row[1]);

        f.render_widget(console_block, console_area);
        f.render_widget(logs_block, logs_area);
        f.render_widget(connections_table, bottom_row[0]);
        f.render_widget(cpu_sparkline, metrics_layout[0]);
        f.render_widget(mem_sparkline, metrics_layout[1]);
    })?;
    Ok(())
}

fn user_info_table(user: &User) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push("+----------------+----------------------+".to_string());
    lines.push(format!("| {:<14} | {:<20}", "Field", "Value"));
    lines.push("+----------------+----------------------+".to_string());
    lines.push(format!("| {:<14} | {:<20}", "Username", user.username));
    lines.push(format!("| {:<14} | {:<20}", "Email", user.email));
    lines.push(format!("| {:<14} | {:<20}", "Balance", user.balance));
    lines.push(format!("| {:<14} | {:<20}", "Password", user.password));
    lines.push("+----------------+----------------------+".to_string());

    lines.join("\n")
}

fn color_for_percentage(value: f32) -> Color {
    match value as u64 {
        0..=50 => Color::Green,
        51..=80 => Color::Yellow,
        _ => Color::Red,
    }
}
