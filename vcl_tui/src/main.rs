mod app;
mod config;
mod connect_action;
mod errors;
mod logging;
mod state;
mod theme;
mod ui;
mod utils;
mod vcl;

use std::io;

use color_eyre::Result;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};

use state::App;

fn main() -> Result<()> {
    let _logging_guard = logging::init();
    errors::install()?;
    let mut app = App::new()?;
    let mut terminal = ratatui::init();
    terminal.clear()?;
    crossterm::execute!(io::stdout(), EnableBracketedPaste)?;
    let result = app.run(&mut terminal);
    let _ = crossterm::execute!(io::stdout(), DisableBracketedPaste);
    ratatui::restore();
    result
}
