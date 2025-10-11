use chrono::{DateTime, Local, Duration};

/// Converts a DateTime<Local> into a human-readable string, such as "5s ago", "1m 3s ago", "1h 2m ago", or "1d 3h ago".
///
/// The function takes a DateTime<Local> as input and returns a string describing the time difference between the input time and the current time.
pub fn humanize_time_since(time: DateTime<Local>) -> String {
    let now = Local::now();
    let delta = now.signed_duration_since(time);

    if delta.num_seconds() < 60 {
        format!("{}s ago", delta.num_seconds())
    } else if delta.num_minutes() < 60 {
        format!("{}m {}s ago", delta.num_minutes(), delta.num_seconds() % 60)
    } else if delta.num_hours() < 24 {
        format!("{}h {}m ago", delta.num_hours(), delta.num_minutes() % 60)
    } else {
        format!("{}d {}h ago", delta.num_days(), delta.num_hours() % 24)
    }
}

/// Converts a byte count into a human-readable string, such as "1 B", "1.5 KB", or "1.23 MB".
pub fn humanize_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
