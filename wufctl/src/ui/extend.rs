use ratatui::{
    Frame,
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
};

use crate::state::{App, EXTEND_PRESETS, ExtendRow, PendingOp};
use crate::theme;

pub fn render_popup(app: &mut App, frame: &mut Frame, id: i64) {
    let area = super::centered_rect(44, 11, frame.area());
    frame.render_widget(Clear, area);
    let block = theme::block().title(format!("Extend Reservation #{id}"));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if matches!(app.pending, Some(PendingOp::ExtendRequest { .. })) {
        super::render_throbber(app, frame, inner, "Extending reservation...");
        return;
    }

    let e = &app.extend;
    let mut lines = vec![
        super::field_line(
            "Extend by",
            e.focus == ExtendRow::Duration,
            EXTEND_PRESETS[e.duration_idx].1.to_string(),
        ),
        Line::from(""),
    ];

    if let Some(msg) = &e.message {
        lines.push(Line::styled(msg.as_str(), theme::danger()));
        lines.push(Line::from(""));
    }

    let confirm_focused = e.focus == ExtendRow::Confirm;
    lines.push(Line::from(vec![Span::styled(
        if confirm_focused {
            "> [ Extend ]"
        } else {
            "  [ Extend ]"
        },
        if confirm_focused {
            theme::accent().add_modifier(Modifier::REVERSED)
        } else {
            theme::accent()
        },
    )]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[Up/Down]", theme::accent()),
        Span::raw(" Field  "),
        Span::styled("[Left/Right]", theme::accent()),
        Span::raw(" Adjust  "),
        Span::styled("[Esc]", theme::accent()),
        Span::raw(" Cancel"),
    ]));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
