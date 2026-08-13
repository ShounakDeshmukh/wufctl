pub mod images;
pub mod reservations;
pub mod setup;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::state::{App, Screen};
use crate::theme;

pub fn draw(app: &mut App, frame: &mut Frame) {
    if app
        .toast
        .as_ref()
        .is_some_and(|t| std::time::Instant::now() >= t.expires_at)
    {
        app.toast = None;
    }

    match app.screen {
        Screen::Setup => setup::render(app, frame),
        Screen::Images => images::render(app, frame),
        Screen::Reservations => reservations::render(app, frame),
        _ => frame.render_widget(Paragraph::new("not yet implemented"), frame.area()),
    }

    render_toast(app, frame);
}

/// Bottom-right, drawn last so it overlays whatever's underneath - matches
/// the approved mockup's own toast (`"> " + msg`), plus a border since it's
/// a floating overlay rather than inline status-bar text like the mockup.
fn render_toast(app: &App, frame: &mut Frame) {
    let Some(toast) = &app.toast else { return };
    let (message, style) = match &toast.text {
        Ok(t) => (t.as_str(), theme::success()),
        Err(t) => (t.as_str(), theme::danger()),
    };
    let text = format!("> {message}");
    let area = frame.area();
    let width = (text.chars().count() as u16 + 2).min(area.width);
    let height = 3.min(area.height);
    let toast_area = Rect {
        x: area.width.saturating_sub(width),
        y: area.height.saturating_sub(height),
        width,
        height,
    };

    frame.render_widget(Clear, toast_area);
    let block = Block::bordered().border_style(style);
    let inner = block.inner(toast_area);
    frame.render_widget(block, toast_area);
    frame.render_widget(
        Paragraph::new(text).style(style.add_modifier(Modifier::BOLD)),
        inner,
    );
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
