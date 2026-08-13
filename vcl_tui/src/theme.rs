//! NC State official brand colors, mapped onto the app's semantic roles.
//! Source: https://brand.ncsu.edu/designing-for-nc-state/color/

use ratatui::style::{Color, Modifier, Style};

pub const WOLFPACK_RED: Color = Color::Rgb(0xcc, 0x00, 0x00);
pub const HUNT_YELLOW: Color = Color::Rgb(0xfa, 0xc8, 0x00);
pub const GENOMIC_GREEN: Color = Color::Rgb(0x6f, 0x7d, 0x1c);
pub const INNOVATION_BLUE: Color = Color::Rgb(0x42, 0x7e, 0x93);
pub const BIO_INDIGO: Color = Color::Rgb(0x41, 0x56, 0xa1);

/// Selection highlight, borders, hint keys, active tab.
pub const ACCENT: Color = BIO_INDIGO;
/// Validation/error text.
pub const DANGER: Color = WOLFPACK_RED;
/// "ready" status.
pub const SUCCESS: Color = GENOMIC_GREEN;
/// "loading"/"pending" status.
pub const PENDING: Color = HUNT_YELLOW;
/// Busy/in-progress state (e.g. "Validating...") and "future" status.
pub const INFO: Color = INNOVATION_BLUE;

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

/// De-emphasized secondary text. Deliberately not a gray `Color`
pub fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}
