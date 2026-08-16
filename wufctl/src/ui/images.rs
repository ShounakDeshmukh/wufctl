use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, List, ListItem, Paragraph, Wrap},
};

use crate::state::{App, PendingOp};
use crate::theme;

/// Big popup over Reservations, only reachable via `n`, not a standalone screen.
pub fn render_popup(app: &mut App, frame: &mut Frame) {
    let full = frame.area();
    let area = super::centered_rect(full.width * 4 / 5, full.height * 4 / 5, full);
    frame.render_widget(Clear, area);

    if let Some(err) = &app.images.error {
        frame.render_widget(
            Paragraph::new(err.as_str())
                .style(theme::danger())
                .block(theme::block().title("Images")),
            area,
        );
        return;
    }

    let [list_area, details_area] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(area);

    render_list(app, frame, list_area);
    render_details(app, frame, details_area);
}

fn render_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let hint = Line::from(vec![
        Span::styled("[/]", theme::accent()),
        Span::raw(" Search   "),
        Span::styled("[Esc]", theme::accent()),
        Span::raw(" Cancel"),
    ]);

    // First load or cache expired: spinner replaces the empty list.
    if app.images.images.is_empty() && matches!(app.pending, Some(PendingOp::LoadImages(_))) {
        let block = theme::block().title("Images").title_bottom(hint);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        super::render_throbber(app, frame, inner, "Loading images...");
        return;
    }

    let visible = app.images.visible_indices();
    let title = if app.images.search.value.is_empty() {
        "Images".to_string()
    } else {
        format!("Images ({}/{})", visible.len(), app.images.images.len())
    };
    let block = theme::block().title(title).title_bottom(hint);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [search_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    render_search_line(app, frame, search_area);

    // Unspaced image names: use char-chunk wrapping.
    let wrap_width = list_area.width.max(1) as usize;
    let items: Vec<ListItem> = visible
        .iter()
        .map(|&i| ListItem::new(wrap_text(&app.images.images[i].name, wrap_width)))
        .collect();
    let list = List::new(items).highlight_style(theme::accent().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, list_area, &mut app.images.list_state);
}

fn render_search_line(app: &App, frame: &mut Frame, area: Rect) {
    if app.images.searching {
        let text = format!("/{}", app.images.search.value);
        frame.render_widget(Paragraph::new(text).style(theme::accent()), area);
        frame.set_cursor_position((area.x + 1 + app.images.search.cursor as u16, area.y));
    } else if !app.images.search.value.is_empty() {
        frame.render_widget(
            Paragraph::new(format!("/{}", app.images.search.value)).style(theme::dim()),
            area,
        );
    } else {
        frame.render_widget(
            Paragraph::new("Press / to search").style(theme::dim()),
            area,
        );
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<Line<'static>> {
    text.chars()
        .collect::<Vec<char>>()
        .chunks(width)
        .map(|chunk| Line::from(chunk.iter().collect::<String>()))
        .collect()
}

fn render_details(app: &App, frame: &mut Frame, area: Rect) {
    let block = theme::block().title("Details");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible = app.images.visible_indices();
    let selected_real_idx = app
        .images
        .list_state
        .selected()
        .and_then(|pos| visible.get(pos).copied());

    let Some(real_idx) = selected_real_idx else {
        let text = if app.images.images.is_empty() {
            "No image selected"
        } else {
            "No images match your search"
        };
        frame.render_widget(Paragraph::new(text), inner);
        return;
    };
    let img = &app.images.images[real_idx];

    let mut lines = if img.is_reservable() {
        vec![
            Line::from(format!("Name: {}", img.name)),
            Line::from(format!("ID: {}", img.id)),
            Line::from(format!("OS: {}", img.ostype)),
            Line::from(format!("Usage: {}", img.usage)),
            Line::from(format!("Description: {}", img.description)),
            Line::from(""),
            Line::from(vec![
                Span::styled("[n]", theme::accent()),
                Span::raw(" "),
                Span::styled("Reserve this image", theme::dim()),
            ]),
        ]
    } else {
        vec![
            Line::from("This is an Azure Virtual Desktop (AVD) resource."),
            Line::from("Not reservable through this tool."),
            Line::from(""),
            Line::from("Press [n] to open NCSU's AVD guide in your browser."),
        ]
    };
    if let Some(msg) = &app.images.message {
        let (text, style) = match msg {
            Ok(text) => (text.as_str(), theme::success()),
            Err(text) => (text.as_str(), theme::danger()),
        };
        lines.push(Line::from(""));
        lines.push(Line::styled(text, style));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
