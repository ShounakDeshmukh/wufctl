mod app;
mod config;
mod errors;
mod logging;
mod utils;

fn main() -> color_eyre::Result<()> {
    let _logging_guard = logging::init();
    errors::install()?;

    let app = app::App::new()?;
    println!("{:?}", app.screen);

    Ok(())
}
