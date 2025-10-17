use chrono::{DateTime, Local};

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

/// Converts a byte count into a human-readable string.
///
/// The function takes a byte count as input and returns a string describing the byte count in a human-readable format, such as "1 B", "1.5 KB", or "1.23 MB".
pub fn humanize_bytes(bytes: u64) -> String {
    const KILOBYTE: u64 = 1024;
    const MEGABYTE: u64 = KILOBYTE * 1024;
    const GIGABYTE: u64 = MEGABYTE * 1024;
    const TERABYTE: u64 = GIGABYTE * 1024;
    const PETABYTE: u64 = TERABYTE * 1024;

    if bytes < KILOBYTE {
        format!("{} B", bytes)
    } else if bytes < MEGABYTE {
        format!("{:.1} KB", (bytes as f64) / KILOBYTE as f64)
    } else if bytes < GIGABYTE {
        format!("{:.2} MB", (bytes as f64) / MEGABYTE as f64)
    } else if bytes < TERABYTE {
        format!("{:.2} GB", (bytes as f64) / GIGABYTE as f64)
    } else if bytes < PETABYTE {
        format!("{:.2} TB", (bytes as f64) / TERABYTE as f64)
    } else {
        format!("{:.2} PB", (bytes as f64) / PETABYTE as f64)
    }
}