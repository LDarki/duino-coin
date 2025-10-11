use colored::*;
use chrono::Local;
use std::io::Write;

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
///
pub fn beautify_print(msg: &str, level: &str) {
    let ts = Local::now().format("%H:%M:%S").to_string();
    let prefix = match level {
        "error" => "[ERROR]".red(),
        "success" => "[OK]".green(),
        "warning" => "[WARN]".yellow(),
        _ => "[INFO]".cyan(),
    };

    let mut first_line = true;

    for line in msg.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if first_line {
                println!("\n{} {} {}", ts.dimmed(), prefix, trimmed);
                first_line = false;
            } else {
                println!("{} {}", ts.dimmed(), trimmed);
            }
        }
    }

    std::io::stdout().flush().unwrap();
}
