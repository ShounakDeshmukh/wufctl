pub mod images;
pub mod setup;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{App, Screen};
use crate::theme;

pub fn draw(app: &mut App, frame: &mut Frame) {
    match app.screen {
        Screen::Setup => setup::render(app, frame),
        Screen::Images => images::render(app, frame),
        _ => frame.render_widget(Paragraph::new("not yet implemented"), frame.area()),
    }
}

/// Shared top bar for every screen (2 rows: text + a rule underneath).
/// Right side is the signed-in indicator - no API call returns the user's
/// own email, so this shows a status dot + "SIGNED IN", not an identity.
pub fn render_top_bar(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::new()
        .borders(Borders::BOTTOM)
        .border_style(theme::dim());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (dot, dot_style, label) = if app.client.is_some() {
        ("\u{25cf}", theme::success(), "SIGNED IN")
    } else {
        ("\u{25cb}", theme::dim(), "NOT SIGNED IN")
    };
    let left = Line::from(vec![
        Span::styled("VCL_TUI", theme::accent().add_modifier(Modifier::BOLD)),
        Span::styled("  NCSU VCL", theme::dim()),
    ]);
    let right = Line::from(vec![
        Span::styled(dot, dot_style),
        Span::raw(" "),
        Span::styled(label, dot_style.add_modifier(Modifier::BOLD)),
    ]);
    let [left_area, right_area] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(right.width() as u16),
    ])
    .areas(inner);

    frame.render_widget(Paragraph::new(left), left_area);
    frame.render_widget(Paragraph::new(right), right_area);
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
