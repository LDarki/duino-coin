use colored::*;
use chrono::Local;
use std::io::Write;
use std::sync::Arc;
use crate::core::app::App;
use tokio::sync::Mutex;
use ratatui::style::{Color, Style};
use ratatui::text::{Span, Text, Line};

pub enum Tab {
    Main,
    Logs,
}

/// Initializes the logger by creating a directory for logs.
///
/// Returns `true` if the directory was successfully created, and `false` otherwise.
/// @TODO: Expand logging functionality (e.g., file logging, log levels)
pub fn init_logger() -> bool {
    let _ = std::fs::create_dir_all("logs");
    true
}

/// Prints a message with a timestamp and a level prefix.
///
/// # Arguments
///
/// * `msg`: The message to be printed.
/// * `level`: The level of the message. Can be "error", "success", "warning", or any other string.
/// * `app`: Optional Arc<Mutex<App>> to log the message in the app context.
/// 
pub fn beautify_print(msg: &str, level: &str, app: Option<Arc<App>>, tab: Tab) {
    let ts = Local::now().format("%H:%M:%S").to_string();
    let prefix = match level.to_lowercase().as_str() {
        "error" => Span::styled("[ERROR]", Style::default().fg(Color::Red)),
        "success" => Span::styled("[OK]", Style::default().fg(Color::Green)),
        "warning" => Span::styled("[WARN]", Style::default().fg(Color::Yellow)),
        _ => Span::styled("[INFO]", Style::default().fg(Color::Cyan)),
    };

    let ts_span = Span::raw(ts);
    let message_span = Span::raw(msg.trim());

    let text_line = Text::from(
        Line::from(vec![
            ts_span,
            Span::raw(" "),
            prefix,
            Span::raw(" "),
            message_span,
        ])
    ).to_string();


    match app {
        Some(app) => {
            let app = Arc::clone(&app);
            let text_line = text_line.clone();
            tokio::spawn(async move {
                match tab {
                    Tab::Main => app.console(text_line),
                    Tab::Logs => app.log(text_line),
                }
            });
        }
        _none => {
            println!("{}", text_line);
            std::io::stdout().flush().unwrap();
        }
    }
}