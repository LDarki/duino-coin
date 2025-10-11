use chrono::{DateTime, Local, Duration};

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

pub fn humanize_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
