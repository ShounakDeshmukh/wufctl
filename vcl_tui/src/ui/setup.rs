use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
};

use crate::app::{App, SetupState};
use crate::theme;

pub fn render(app: &App, frame: &mut Frame) {
    let [top_bar_area, pane_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(frame.area());

    render_top_bar(frame, top_bar_area);

    let card_area = super::centered_rect(60, 16, pane_area);
    let block = Block::bordered().title("Setup");
    let inner = block.inner(card_area);
    frame.render_widget(block, card_area);

    let [
        no_token,
        _spacer1,
        token_label,
        input_area,
        helper,
        _spacer2,
        status,
        hint,
        _spacer3,
        footer,
        _rest,
    ] = Layout::vertical([
        Constraint::Length(1), // "No VCL API token found."
        Constraint::Length(1), // spacer
        Constraint::Length(1), // "Token" label
        Constraint::Length(3), // input box (bordered)
        Constraint::Length(3), // helper text (3 steps)
        Constraint::Length(1), // spacer
        Constraint::Length(1), // status line
        Constraint::Length(1), // hint line
        Constraint::Length(1), // spacer
        Constraint::Length(1), // footer
        Constraint::Min(0),
    ])
    .areas(inner);

    frame.render_widget(
        Paragraph::new("No VCL API token found.").style(theme::dim()),
        no_token,
    );
    frame.render_widget(Paragraph::new("Token").style(theme::dim()), token_label);

    render_input(app, frame, input_area);

    let steps = Text::from(vec![
        Line::raw("1. Log in to https://vcl.ncsu.edu"),
        Line::raw("2. Manage -> User Preferences -> Manage Tokens"),
        Line::raw("3. Generate a new token, then copy it here"),
    ]);
    frame.render_widget(Paragraph::new(steps).style(theme::dim()), helper);

    render_status(app, frame, status);

    frame.render_widget(
        Line::from(vec![
            Span::styled("[Enter]", theme::accent()),
            Span::raw(" "),
            Span::styled("Validate & continue", theme::dim()),
            Span::raw("   "),
            Span::styled("[Ctrl+R]", theme::accent()),
            Span::raw(" "),
            Span::styled(
                if app.setup.masked {
                    "Show token"
                } else {
                    "Hide token"
                },
                theme::dim(),
            ),
        ]),
        hint,
    );

    render_footer(frame, footer);
}

fn render_top_bar(frame: &mut Frame, area: ratatui::layout::Rect) {
    let right_text = "\u{25cb} not signed in";

    let [left_area, right_area] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(right_text.chars().count() as u16),
    ])
    .areas(area);

    frame.render_widget(
        Paragraph::new("vcl_tui  NCSU VCL").style(theme::dim()),
        left_area,
    );
    frame.render_widget(Paragraph::new(right_text).style(theme::dim()), right_area);
}

fn render_input(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let display = if app.setup.masked {
        "\u{2022}".repeat(app.setup.input.chars().count())
    } else {
        app.setup.input.clone()
    };
    let input_block = Block::bordered().border_style(theme::accent());
    let content_area = input_block.inner(area);
    frame.render_widget(input_block, area);

    // Scroll window follows the cursor (not the string end) so it never
    // overflows the border and Left/Right still works once scrolled.
    let width = content_area.width as usize;
    let total = display.chars().count();
    let cursor = app.setup.cursor.min(total);
    let start = cursor.saturating_sub(width.saturating_sub(1));
    let visible: String = display.chars().skip(start).take(width).collect();
    let cursor_col = cursor - start;

    frame.render_widget(Paragraph::new(visible), content_area);

    frame.set_cursor_position((content_area.x + cursor_col as u16, content_area.y));
}

fn render_status(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let (text, style) = match &app.setup.state {
        SetupState::Idle => (String::new(), Style::default()),
        SetupState::Validating => ("Validating token...".to_string(), theme::info()),
        SetupState::Error(msg) => (msg.clone(), theme::danger()),
    };
    frame.render_widget(Paragraph::new(text).style(style), area);
}

fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect) {
    let text = match crate::utils::get_config_dir() {
        Some(dir) => format!("Saved to {}/config.toml (chmod 600)", dir.display()),
        None => "Saved to your config directory as config.toml (chmod 600)".to_string(),
    };
    frame.render_widget(Paragraph::new(text).style(theme::dim()), area);
}
