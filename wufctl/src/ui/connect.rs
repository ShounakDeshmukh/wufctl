use ratatui::{
    Frame,
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
};

use crate::state::{App, PendingOp};
use crate::theme;

fn kv_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {label:<10}"), theme::dim()),
        Span::raw(value.to_string()),
    ])
}

pub fn render_popup(app: &mut App, frame: &mut Frame, id: i64) {
    let area = super::centered_rect(64, 20, frame.area());
    frame.render_widget(Clear, area);
    let block = theme::block().title(format!("Connect - Reservation #{id}"));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if matches!(app.pending, Some(PendingOp::LoadConnectData(_))) {
        super::render_throbber(app, frame, inner, "Fetching connection info...");
        return;
    }

    let mut lines = vec![Line::from("")];

    if let Some(data) = &app.connect.data {
        lines.push(kv_line("Host", &data.server_ip));
        lines.push(kv_line("User", &data.user));
        lines.push(kv_line("Password", &data.password));
        lines.push(kv_line("Port", &data.connect_port));
        lines.push(Line::from(""));

        lines.push(Line::styled("  Methods", theme::dim()));
        for method in &data.connect_methods {
            lines.push(Line::from(format!(
                "    {} ({})",
                method.description,
                method.connectports.join(", ")
            )));
        }
        lines.push(Line::from(""));

        lines.push(Line::styled("  Command", theme::dim()));
        lines.push(Line::from(format!(
            "    ssh {}@{} -p {}",
            data.user, data.server_ip, data.connect_port
        )));
    } else if let Some(err) = &app.connect.error {
        lines.push(Line::styled(format!("  {err}"), theme::danger()));
    }

    lines.push(Line::from(""));
    if let Some(data) = &app.connect.data {
        let mut hints = vec![("  [Enter]", "Connect")];
        if data
            .connect_methods
            .iter()
            .any(|m| m.description.to_lowercase().contains("rdp"))
        {
            hints.push(("[r]", "Save RDP file"));
        }
        hints.push(("[Esc]", "Back"));
        lines.push(super::hint_line(&hints));
    } else {
        lines.push(Line::styled("  [Esc] Back", theme::dim()));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
