use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, Paragraph, Wrap},
};
use throbber_widgets_tui::Throbber;

use crate::state::{App, PendingOp};
use crate::theme;
use crate::utils;

pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    if let Some(err) = &app.reservations.error {
        frame.render_widget(
            Paragraph::new(err.as_str())
                .style(theme::danger())
                .block(theme::block().title("Reservations")),
            area,
        );
        return;
    }

    let [list_area, details_area] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(area);

    render_list(app, frame, list_area);
    render_details(app, frame, details_area);
}

fn status_style(status: &str) -> Style {
    match status {
        "ready" => theme::success(),
        "loading" => theme::pending(),
        _ => theme::info(),
    }
}

fn render_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let hint = Line::from(vec![
        Span::styled("[n]", theme::accent()),
        Span::raw(" New reservation   "),
        Span::styled("[r]", theme::accent()),
        Span::raw(" Refresh   "),
        Span::styled("[t]", theme::accent()),
        Span::raw(" Change token   "),
        Span::styled("[q]", theme::accent()),
        Span::raw(" Quit"),
    ]);
    let block = theme::block().title("Reservations").title_bottom(hint);

    // First load or cache expired: spinner replaces the empty list.
    if app.reservations.reservations.is_empty()
        && matches!(app.pending, Some(PendingOp::LoadReservations(_)))
    {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        super::render_throbber(app, frame, inner, "Loading reservations...");
        return;
    }

    let items: Vec<ListItem> = app
        .reservations
        .reservations
        .iter()
        .map(|r| {
            let mut spans = vec![Span::raw(r.image_name.clone()), Span::raw(" ")];
            // Animates while a refresh is in flight, sits on one frame between polls.
            if r.status.status == "loading" {
                spans.push(
                    Throbber::default()
                        .throbber_style(theme::pending())
                        .to_symbol_span(&app.throbber_state),
                );
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                format!("[{}]", r.status.status),
                status_style(&r.status.status),
            ));
            ListItem::new(Line::from(spans))
        })
        .collect();
    let list = List::new(items)
        .block(block)
        .highlight_style(theme::accent().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut app.reservations.list_state);
}

fn render_details(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = theme::block().title("Details");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(i) = app.reservations.list_state.selected() else {
        frame.render_widget(
            Paragraph::new("You have no current reservations.").style(theme::dim()),
            inner,
        );
        return;
    };

    if matches!(app.pending, Some(PendingOp::EndReservation { .. })) {
        super::render_throbber(app, frame, inner, "Ending reservation...");
        return;
    }

    let r = &app.reservations.reservations[i];

    let mut lines = vec![
        Line::from(format!("ID: #{}", r.id)),
        Line::from(format!("Image: {}", r.image_name)),
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(&r.status.status, status_style(&r.status.status)),
        ]),
        Line::from(format!("Starting: {}", utils::format_timestamp(r.start))),
        Line::from(format!("Ending: {}", utils::format_timestamp(r.end))),
    ];

    lines.push(Line::from(""));
    match r.status.status.as_str() {
        "ready" => {
            lines.push(super::hint_line(&[("[c]", "Connect"), ("[e]", "Extend")]));
            lines.push(super::hint_line(&[("[x]", "End reservation")]));
        }
        "loading" => {
            lines.push(Line::styled("Provisioning image...", theme::dim()));
            lines.push(Line::styled(
                "This page updates automatically every 20s until it's ready.",
                theme::dim(),
            ));
        }
        _ => lines.push(super::hint_line(&[("[x]", "End reservation")])),
    }

    if let Some(msg) = &app.reservations.message {
        let (text, style) = super::result_parts(msg);
        lines.push(Line::from(""));
        lines.push(Line::styled(text, style));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

pub fn render_confirm_end_popup(app: &App, frame: &mut Frame, index: usize) {
    let r = &app.reservations.reservations[index];
    render_confirm_popup(
        frame,
        "End Reservation",
        format!("End reservation #{} ({})?", r.id, r.image_name),
        theme::danger(),
    );
}

pub fn render_confirm_change_token_popup(frame: &mut Frame) {
    render_confirm_popup(
        frame,
        "Change Token",
        "Change the saved API token?".to_string(),
        theme::accent(),
    );
}

/// Shared yes/no card; `confirm_style` colors [Enter] red for destructive prompts.
fn render_confirm_popup(
    frame: &mut Frame,
    title: &'static str,
    prompt: String,
    confirm_style: Style,
) {
    let area = super::centered_rect(46, 8, frame.area());
    frame.render_widget(Clear, area);
    let block = theme::block().title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(prompt),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Enter]", confirm_style),
            Span::raw(" "),
            Span::styled("Confirm", theme::dim()),
            Span::raw("   "),
            Span::styled("[Esc]", theme::accent()),
            Span::raw(" "),
            Span::styled("Cancel", theme::dim()),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
