use std::path::PathBuf;

pub fn get_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("vcl_tui"))
}

/// Converts a char index into the byte index `String::insert`/`remove`
/// need, since input may contain multi-byte UTF-8 characters.
pub fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}
