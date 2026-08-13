//! Terminal-native colors, mapped onto the app's semantic roles. Uses the
//! ANSI palette (`Color::Red`/`Green`/etc.) rather than fixed RGB values,
//! so the app follows whatever theme the user's terminal defines instead
//! of a hardcoded look.

use ratatui::style::{Color, Modifier, Style};

/// Selection highlight, borders, hint keys, active tab.
pub const ACCENT: Color = Color::Blue;
/// Validation/error text.
pub const DANGER: Color = Color::Red;
/// "ready" status.
pub const SUCCESS: Color = Color::Green;
/// "loading"/"pending" status.
pub const PENDING: Color = Color::Yellow;
/// Busy/in-progress state (e.g. "Validating...") and "future" status.
pub const INFO: Color = Color::Cyan;

pub fn accent() -> Style {
    Style::default().fg(ACCENT)
}

pub fn danger() -> Style {
    Style::default().fg(DANGER)
}

pub fn success() -> Style {
    Style::default().fg(SUCCESS)
}

pub fn pending() -> Style {
    Style::default().fg(PENDING)
}

pub fn info() -> Style {
    Style::default().fg(INFO)
}

/// De-emphasized secondary text - the terminal's own dim/faint rendering
/// attribute rather than a specific gray, so it works with any theme.
pub fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}
