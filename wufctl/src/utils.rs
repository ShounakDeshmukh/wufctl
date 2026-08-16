use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use chrono_tz::Tz;

pub fn get_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("wufctl"))
}

/// `chrono::Local` can't resolve a named zone like "EDT" on its own, so look up the system's
/// IANA zone via `iana-time-zone` and format through `chrono-tz` instead.
pub fn format_timestamp(unix_secs: i64) -> String {
    let Some(dt) = Utc.timestamp_opt(unix_secs, 0).single() else {
        return "unknown".to_string();
    };
    match iana_time_zone::get_timezone()
        .ok()
        .and_then(|name| name.parse::<Tz>().ok())
    {
        Some(tz) => dt
            .with_timezone(&tz)
            .format("%A, %b %-d, %Y, %-I:%M %p %Z")
            .to_string(),
        None => dt.format("%A, %b %-d, %Y, %-I:%M %p UTC").to_string(),
    }
}

/// Converts a char index into the byte index `String::insert`/`remove` need.
pub fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}
