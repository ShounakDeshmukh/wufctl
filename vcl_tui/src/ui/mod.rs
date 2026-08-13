pub mod setup;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::Paragraph,
};

use crate::app::{App, Screen};

pub fn draw(app: &App, frame: &mut Frame) {
    match app.screen {
        Screen::Setup => setup::render(app, frame),
        _ => frame.render_widget(Paragraph::new("not yet implemented"), frame.area()),
    }
}

/// Centers a `width` x `height` rect within `area`, clamped to its bounds.
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let [_, vert, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [_, rect, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width),
        Constraint::Fill(1),
    ])
    .areas(vert);
    rect
}
