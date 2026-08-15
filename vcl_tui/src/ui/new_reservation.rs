use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};

use crate::state::{AmPm, App, DURATION_PRESETS, FormRow, MINUTE_STEPS, PendingOp, StartChoice};
use crate::theme;

pub fn render_popup(app: &mut App, frame: &mut Frame, image_idx: usize) {
    let area = super::centered_rect(56, 20, frame.area());
    frame.render_widget(Clear, area);
    let block = theme::block().title("New Reservation");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if matches!(app.pending, Some(PendingOp::AddRequest(_))) {
        super::render_throbber(app, frame, inner, "Creating reservation...");
        return;
    }

    let f = &app.new_reservation;
    let image_name = app
        .images
        .images
        .get(image_idx)
        .map(|img| img.name.as_str())
        .unwrap_or("(unknown)");

    let mut lines = vec![
        Line::from(format!("Image: {image_name}")),
        Line::from(""),
        field_line(
            "Start",
            f.focus == FormRow::Start,
            match f.start {
                StartChoice::Now => "Now".to_string(),
                StartChoice::Later => "Later".to_string(),
            },
        ),
    ];

    if f.start == StartChoice::Later {
        lines.push(field_line(
            "Day",
            f.focus == FormRow::Day,
            day_label(f.day_offset),
        ));
        lines.push(field_line(
            "Hour",
            f.focus == FormRow::Hour,
            f.hour.to_string(),
        ));
        lines.push(field_line(
            "Minute",
            f.focus == FormRow::Minute,
            format!("{:02}", MINUTE_STEPS[f.minute_idx as usize]),
        ));
        lines.push(field_line(
            "AM/PM",
            f.focus == FormRow::AmPm,
            match f.am_pm {
                AmPm::Am => "AM".to_string(),
                AmPm::Pm => "PM".to_string(),
            },
        ));
    }

    let is_custom = f.duration_idx == DURATION_PRESETS.len();
    lines.push(field_line(
        "Duration",
        f.focus == FormRow::Duration,
        if is_custom {
            "Custom".to_string()
        } else {
            DURATION_PRESETS[f.duration_idx].1.to_string()
        },
    ));

    let mut custom_minutes_cursor = None;
    if is_custom {
        let focused = f.focus == FormRow::CustomMinutes;
        let marker = if focused { "> " } else { "  " };
        let prefix = format!("{marker}Minutes : ");
        if focused {
            custom_minutes_cursor = Some((prefix.chars().count(), lines.len()));
        }
        lines.push(Line::from(vec![
            Span::raw(prefix),
            Span::styled(
                f.custom_minutes.clone(),
                if focused {
                    theme::accent()
                } else {
                    Style::default()
                },
            ),
        ]));
    }

    lines.push(Line::from(""));
    if let Some(msg) = &f.message {
        lines.push(Line::styled(msg.as_str(), theme::danger()));
        lines.push(Line::from(""));
    }

    let create_focused = f.focus == FormRow::Create;
    lines.push(Line::from(vec![Span::styled(
        if create_focused {
            "> [ Create Reservation ]"
        } else {
            "  [ Create Reservation ]"
        },
        if create_focused {
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
        Span::raw(" Back"),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);

    if let Some((col, row)) = custom_minutes_cursor {
        frame.set_cursor_position((
            inner.x + col as u16 + f.custom_cursor as u16,
            inner.y + row as u16,
        ));
    }
}

/// An actual calendar date reads far less ambiguously than a bare "+2"
/// offset - e.g. "Sat Aug 17" rather than making the user do day-of-week
/// math in their head from a plain day-count.
fn day_label(offset: u8) -> String {
    let date = chrono::Local::now().date_naive() + chrono::Days::new(offset as u64);
    let formatted = date.format("%a %b %-d").to_string();
    if offset == 0 {
        format!("{formatted} (Today)")
    } else {
        formatted
    }
}

/// One "> Label : < value >" row - the spinner-style affordance signals
/// `Left`/`Right` cycles it; unfocused rows drop the marker and brackets.
fn field_line(label: &str, focused: bool, value: String) -> Line<'static> {
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
