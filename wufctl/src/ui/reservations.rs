use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, Paragraph, Wrap},
};

use crate::state::{App, PendingOp};
use crate::theme;
use crate::utils;

pub fn render(app: &mut App, frame: &mut Frame) {
    let [top_bar_area, pane_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(frame.area());

    super::render_top_bar(app, frame, top_bar_area);

    if let Some(err) = &app.reservations.error {
        frame.render_widget(
            Paragraph::new(err.as_str())
                .style(theme::danger())
                .block(theme::block().title("Reservations")),
            pane_area,
        );
        return;
    }

    let [list_area, details_area] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .areas(pane_area);

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
            ListItem::new(Line::from(vec![
                Span::raw(r.image_name.clone()),
                Span::raw(" "),
                Span::styled(
                    format!("[{}]", r.status.status),
                    status_style(&r.status.status),
                ),
            ]))
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
            lines.push(Line::from(vec![
                Span::styled("[c]", theme::accent()),
                Span::raw(" "),
                Span::styled("Connect", theme::dim()),
                Span::raw("   "),
                Span::styled("[e]", theme::accent()),
                Span::raw(" "),
                Span::styled("Extend", theme::dim()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("[x]", theme::accent()),
                Span::raw(" "),
                Span::styled("End reservation", theme::dim()),
            ]));
        }
        "loading" => {
            lines.push(Line::styled("Provisioning image...", theme::dim()));
            lines.push(Line::styled(
                "This page updates automatically every 20s until it's ready.",
                theme::dim(),
            ));
        }
        _ => {
            lines.push(Line::from(vec![
                Span::styled("[x]", theme::accent()),
                Span::raw(" "),
                Span::styled("End reservation", theme::dim()),
            ]));
        }
    }

    if let Some(msg) = &app.reservations.message {
        let (text, style) = match msg {
            Ok(text) => (text.as_str(), theme::success()),
            Err(text) => (text.as_str(), theme::danger()),
        };
        lines.push(Line::from(""));
        lines.push(Line::styled(text, style));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

pub fn render_confirm_end_popup(app: &App, frame: &mut Frame, index: usize) {
    let area = super::centered_rect(46, 8, frame.area());
    frame.render_widget(Clear, area);
    let block = theme::block().title("End Reservation");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let r = &app.reservations.reservations[index];
    let lines = vec![
        Line::from(""),
        Line::from(format!("End reservation #{} ({})?", r.id, r.image_name)),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Enter]", theme::danger()),
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

pub fn render_confirm_change_token_popup(frame: &mut Frame) {
    let area = super::centered_rect(46, 8, frame.area());
    frame.render_widget(Clear, area);
    let block = theme::block().title("Change Token");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from("Change the saved API token?"),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Enter]", theme::accent()),
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
