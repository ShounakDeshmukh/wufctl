mod app;
mod config;
mod errors;
mod logging;
mod theme;
mod ui;
mod utils;

fn main() -> color_eyre::Result<()> {
    let _logging_guard = logging::init();
    errors::install()?;
    let mut app = app::App::new()?;
    let mut terminal = ratatui::init();
    terminal.clear()?;
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableBracketedPaste)?;
    let result = app.run(&mut terminal);
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableBracketedPaste);
    ratatui::restore();
    result
}
