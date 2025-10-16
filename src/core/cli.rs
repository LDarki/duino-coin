use std::io::{self, Stdout};
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
    widgets::{Block, Borders, Paragraph, Wrap, Table, Row, Sparkline},
    style::{Color, Style},
    text::{Span, Text},
    Terminal,
};
use tokio::sync::Mutex;
use tokio::time::sleep;
use sysinfo::{System};

use crate::{
    core::app::App, 
    core::app::Connection,
    core::db::Databases,
    models::user::UserModel,
    models::user::User,
    utils::helpers::*,
};

pub async fn start_cli(dbs: Databases, app: Arc<Mutex<App>>) -> anyhow::Result<()> {
    let user_model = Arc::new(UserModel::new(dbs.users.clone()));
    let mut sys = System::new_all();

    {
        let mut app_lock = app.lock().await;
        app_lock.console(colored::Colorize::yellow("Type 'help' for commands, 'exit' to quit.").to_string());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut restart_confirmation_pending = false;

    loop {
        let (
            _input_text,
            connections,
            logs_text,
            console_text,
            cpu_history,
            mem_history
        ) = {
            let mut app_lock = app.lock().await;
            let logs_str = app_lock.logs.iter().cloned().collect::<Vec<_>>().join("\n");

            let console_content = {
                let mut lines = app_lock.console.iter().map(|s| s.trim_end()).collect::<Vec<_>>();

                let prompt_line = Text::from(Line::from(
                    vec![
                        Span::styled("DUCO Server $", Style::default().fg(Color::Yellow)),
                        Span::raw(" "),
                        Span::raw(&app_lock.input),
                    ]
                )).to_string();

                lines.push(&prompt_line);
                lines.join("\n")
            };

            sys.refresh_cpu_all();
            sys.refresh_memory();

            let global_cpu = sys.global_cpu_usage();
            let total_mem = sys.total_memory() as f32;
            let used_mem = sys.used_memory() as f32;
            let mem_percent: f32 = (used_mem / total_mem) * 100.0;

            app_lock.cpu_history.push(global_cpu);
            app_lock.mem_history.push(mem_percent);
            if app_lock.cpu_history.len() > 30 {
                app_lock.cpu_history.remove(0);
            }
            if app_lock.mem_history.len() > 30 {
                app_lock.mem_history.remove(0);
            }

            (
                app_lock.input.clone(),
                app_lock.connections.clone(),
                logs_str,
                console_content,
                app_lock.cpu_history.clone(),
                app_lock.mem_history.clone(),
            )
        };

        draw_tui(
            &mut terminal,
            &_input_text,
            &connections,
            &logs_text,
            &console_text,
            &cpu_history,
            &mem_history,
        )?;

        sleep(Duration::from_millis(100)).await;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                let mut should_break = false;

                match key.code {
                    KeyCode::Char(c) => {
                        let mut app_lock = app.lock().await;
                        if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
                            app_lock.console(colored::Colorize::yellow("Received Ctrl+C. Exiting...").to_string());
                            should_break = true;
                        } else {
                            app_lock.input.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        let mut app_lock = app.lock().await;
                        app_lock.input.pop();
                    }
                    KeyCode::Enter => {
                        let cmd = {
                            let mut app_lock = app.lock().await;
                            let cmd = app_lock.input.trim().to_string();
                            app_lock.input.clear();
                            cmd
                        };

                        if cmd.is_empty() {
                            continue;
                        }

                        if restart_confirmation_pending {
                            let mut app_lock = app.lock().await;
                            if cmd.to_lowercase() == "y" {
                                app_lock.console(colored::Colorize::green("Restarting...").to_string());
                                *app_lock = App::new();
                            } else {
                                app_lock.console(colored::Colorize::yellow("Restart cancelled.").to_string());
                            }
                            restart_confirmation_pending = false;
                            continue;
                        }

                        {
                            let mut app_lock = app.lock().await;
                            app_lock.console(format!("> {}", cmd));
                        }

                        match cmd.as_str() {
                            "exit" => {
                                let mut app_lock = app.lock().await;
                                app_lock.log(colored::Colorize::red("Exiting CLI loop.").to_string());
                                should_break = true;
                            }
                            "clear" => {
                                let mut app_lock = app.lock().await;
                                app_lock.console.clear();
                            }
                            "restart" => {
                                let mut app_lock = app.lock().await;
                                app_lock.console("Are you sure you want to restart? (Y/n)".to_string());
                                restart_confirmation_pending = true;
                            }
                            _ => {
                                handle_command(cmd, Arc::clone(&app), Arc::clone(&user_model)).await;
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

async fn handle_command(cmd: String, app: Arc<Mutex<App>>, user_model: Arc<UserModel>) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    match parts.get(0).map(|s| *s) {
        Some("help") => {
            let help_msg = "Commands Available:
            - exit: Quits the TUI
            - help: Shows this message
            - restart: Re-initializes server state (needs 'Y' confirmation)
            - clear
            - user add/info/del/upd";
            let mut app = app.lock().await;
            app.console(help_msg.to_string());
        }
        Some("user") => {
            handle_user_command(parts, app, user_model).await;
        }
        _ => {
            let mut app = app.lock().await;
            app.console("Unknown command. Type 'help' for a list of commands.".to_string());
        }
    }
}

async fn handle_user_command(
    parts: Vec<&str>,
    app: Arc<Mutex<App>>,
    user_model: Arc<UserModel>,
) {
    let app_clone = Arc::clone(&app);
    let user_model_clone = Arc::clone(&user_model);

    match parts.get(1).map(|s| *s) {
        Some("add") if parts.len() == 6 => {
            let username = parts[2].to_string();
            let password = parts[3].to_string();
            let email = parts[4].to_string();
            let balance: f64 =
                parts[5].parse().unwrap_or(0.0);

            match user_model_clone
                .add_user(&username, &password, &email, balance)
                .await 
            {
                Ok(_) => {
                    let mut app = app_clone.lock().await; 
                    app.console(
                        format!("User added: {}", username)
                            .green()
                            .to_string(),
                    );
                }
                Err(e) => {
                    let mut app = app_clone.lock().await;
                    app.console(
                        format!("Error: {}", e).red().to_string(),
                    );
                }
            }
        }
        Some("info") if parts.len() == 3 => {
            let username = parts[2];
            match user_model_clone.get_user(username).await { 
                Ok(Some(u)) => {
                    let mut app = app_clone.lock().await;
                    app.console(user_info_table(&u));
                }
                Ok(none) => {
                    let mut app = app_clone.lock().await;
                    app.console(
                        format!("User '{}' not found", username)
                            .yellow()
                            .to_string(),
                    );
                }
                Err(e) => {
                    let mut app = app_clone.lock().await;
                    app.console(
                        format!("Error: {:?}", e).red().to_string(),
                    );
                }
            }
        }
        Some("del") if parts.len() == 3 => {
            let username = parts[2];
            match user_model_clone.delete_user(username).await { 
                Ok(_) => {
                    let mut app = app_clone.lock().await; 
                    app.console(
                        format!("User deleted: {}", username)
                            .green()
                            .to_string(),
                    );
                }
                Err(e) => {
                    let mut app = app_clone.lock().await;
                    app.console(
                        format!("Delete failed: {}", e)
                            .red()
                            .to_string(),
                    );
                }
            }
        }
        Some("upd") if parts.len() == 5 => {
            let username = parts[2];
            let field = parts[3];
            let value = parts[4];
            match user_model_clone
                .update_user(username, field, value)
                .await 
            {
                Ok(_) => {
                    let mut app = app_clone.lock().await; 
                    app.console(
                        format!(
                            "User {} updated field {} to {}",
                            username, field, value
                        )
                        .green()
                        .to_string(),
                    );
                }
                Err(e) => {
                    let mut app = app_clone.lock().await;
                    app.console(
                        format!("Update failed: {}", e)
                            .red()
                            .to_string(),
                    );
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
                ";
            let mut app = app_clone.lock().await; 
            app.console(usage.to_string());
        }
    }
}

pub fn draw_tui(
    terminal: &mut Terminal<CrosstermBackend<&mut Stdout>>,
    _input_text: &str,
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
        } else { 0 };

        let console_block = Paragraph::new(console_text)
            .block(Block::default().title("DUCO Server $").borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .scroll((console_scroll as u16, 0));

        let logs_area = top_row[1];
        let logs_height = logs_area.height.saturating_sub(2);
        let logs_line_count = logs_text.lines().count() as u16;
        let logs_scroll = if logs_line_count > logs_height {
            logs_line_count - logs_height
        } else { 0 };

        let logs_block = Paragraph::new(logs_text)
            .block(Block::default().title("Logs").borders(Borders::ALL))
            .wrap(Wrap { trim: true })
            .scroll((logs_scroll as u16, 0));

        let table_rows: Vec<Row> = connections.iter().map(|c| {
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
        }).collect();

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
                "Name", "IP", "Port", "Status", "Sent", "Received", "Uptime"
            ]))
            .block(Block::default().title("Connections").borders(Borders::ALL));

        let cpu_last = *cpu_history.last().unwrap_or(&0.0);
        let cpu_sparkline = Sparkline::default()
            .block(
                Block::default()
                    .title(format!("CPU Usage: {:.1}%", cpu_last))
                    .borders(Borders::ALL)
            )
            .data(&cpu_history.iter().map(|v| *v as u64).collect::<Vec<u64>>())
            .style(Style::default().fg(color_for_percentage(cpu_last)))
            .max(100);

        let mem_last = *mem_history.last().unwrap_or(&0.0);
        let mem_sparkline = Sparkline::default()
            .block(
                Block::default()
                    .title(format!("Memory Usage: {:.1}%", mem_last))
                    .borders(Borders::ALL)
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
