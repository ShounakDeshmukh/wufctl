use std::path::PathBuf;

pub fn get_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("vcl_tui"))
}

/// Formats a Unix timestamp like the web UI's "Saturday, Aug 15, 2026, 4:13
/// PM -04:00" - `%:z` gives a numeric offset since `chrono::Local` on this
/// system doesn't resolve named zone abbreviations (e.g. "EDT") from `%Z`.
pub fn format_timestamp(unix_secs: i64) -> String {
    use chrono::{Local, TimeZone};
    match Local.timestamp_opt(unix_secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%A, %b %-d, %Y, %-I:%M %p %:z").to_string(),
        _ => "unknown".to_string(),
    }
}

/// Converts a char index into the byte index `String::insert`/`remove`
/// need, since input may contain multi-byte UTF-8 characters.
pub fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}
