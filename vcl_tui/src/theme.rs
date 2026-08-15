//! Uses the ANSI palette
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType};

pub const ACCENT: Color = Color::Blue;
pub const DANGER: Color = Color::Red;
pub const SUCCESS: Color = Color::Green;
pub const PENDING: Color = Color::Yellow;
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

/// Terminal DIM
pub fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// All panels use the same rounded border style
pub fn block<'a>() -> Block<'a> {
    Block::bordered().border_type(BorderType::Rounded)
}
