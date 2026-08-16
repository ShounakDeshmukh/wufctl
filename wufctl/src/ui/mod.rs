pub mod connect;
pub mod extend;
pub mod images;
pub mod new_reservation;
pub mod reservations;
pub mod setup;

use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use throbber_widgets_tui::Throbber;

use crate::state::{App, Popup, Screen};
use crate::theme;

pub fn draw(app: &mut App, frame: &mut Frame) {
    if app
        .toast
        .as_ref()
        .is_some_and(|t| Instant::now() >= t.expires_at)
    {
        app.toast = None;
    }
    // Advances one frame per draw, so it keeps spinning while the background call blocks.
    if app.pending.is_some() {
        app.throbber_state.calc_next();
    }

    match app.screen {
        Screen::Setup => setup::render(app, frame),
        Screen::Reservations => reservations::render(app, frame),
    }

    // Popups draw on top of whatever screen is underneath, which keeps rendering.
    match app.popup {
        Popup::None => {}
        Popup::ImagePicker => images::render_popup(app, frame),
        Popup::NewReservationForm { image_idx } => {
            new_reservation::render_popup(app, frame, image_idx)
        }
        Popup::ExtendForm { id } => extend::render_popup(app, frame, id),
        Popup::Connect { id } => connect::render_popup(app, frame, id),
        Popup::ConfirmEnd { index } => reservations::render_confirm_end_popup(app, frame, index),
    }

    render_toast(app, frame);
}

/// One "> Label : < value >" row for a spinner-style form field, shared by both popups.
pub fn field_line(label: &str, focused: bool, value: String) -> Line<'static> {
    let marker = if focused { "> " } else { "  " };
    let value_text = if focused {
        format!("< {value} >")
    } else {
        value
    };
    let style = if focused {
        theme::accent()
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::raw(format!("{marker}{label:<8}: ")),
        Span::styled(value_text, style),
    ])
}

/// Centered throbber + label, drawn inside whichever panel is waiting on content.
pub fn render_throbber(app: &mut App, frame: &mut Frame, area: Rect, label: &'static str) {
    let text_width = (label.chars().count() as u16 + 2).min(area.width);
    let row = Rect {
        x: area.x + area.width.saturating_sub(text_width) / 2,
        y: area.y + area.height / 2,
        width: text_width,
        height: 1.min(area.height),
    };
    let throbber = Throbber::default()
        .label(label)
        .throbber_style(theme::info());
    frame.render_stateful_widget(throbber, row, &mut app.throbber_state);
}

/// Bottom-right, drawn last so it overlays whatever's underneath.
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
    let block = theme::block().border_style(style);
    let inner = block.inner(toast_area);
    frame.render_widget(block, toast_area);
    frame.render_widget(
        Paragraph::new(text).style(style.add_modifier(Modifier::BOLD)),
        inner,
    );
}

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
        Span::styled("WUFCTL", theme::accent().add_modifier(Modifier::BOLD)),
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
