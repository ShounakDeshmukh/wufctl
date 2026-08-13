use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::Line,
    widgets::{Block, List, ListItem, Paragraph},
};

use crate::state::App;
use crate::theme;

pub fn render(app: &mut App, frame: &mut Frame) {
    let [top_bar_area, pane_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(frame.area());

    super::render_top_bar(app, frame, top_bar_area);

    if let Some(err) = &app.images.error {
        frame.render_widget(
            Paragraph::new(err.as_str())
                .style(theme::danger())
                .block(Block::bordered().title("Images")),
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

fn render_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::bordered().title("Images");
    // Image names are often a single unspaced token, so a plain char-chunk
    // wrap (not word-wrap) is what actually keeps a long name on-screen.
    let wrap_width = block.inner(area).width.max(1) as usize;
    let items: Vec<ListItem> = app
        .images
        .images
        .iter()
        .map(|img| ListItem::new(wrap_text(&img.name, wrap_width)))
        .collect();
    let list = List::new(items)
        .block(block)
        .highlight_style(theme::accent().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut app.images.list_state);
}

fn wrap_text(text: &str, width: usize) -> Vec<Line<'static>> {
    text.chars()
        .collect::<Vec<char>>()
        .chunks(width)
        .map(|chunk| Line::from(chunk.iter().collect::<String>()))
        .collect()
}

fn render_details(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::bordered().title("Details");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(i) = app.images.list_state.selected() else {
        frame.render_widget(Paragraph::new("No image selected"), inner);
        return;
    };
    let img = &app.images.images[i];

    let mut lines = if img.is_reservable() {
        vec![
            Line::from(format!("Name: {}", img.name)),
            Line::from(format!("ID: {}", img.id)),
            Line::from(format!("OS: {}", img.ostype)),
            Line::from(format!("Usage: {}", img.usage)),
            Line::from(format!("Description: {}", img.description)),
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
    frame.render_widget(Paragraph::new(lines), inner);
}
