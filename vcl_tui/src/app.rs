use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Setup,
    Images,
    Reservations,
    Connect,
}

pub struct App {
    pub screen: Screen,
    pub config: Option<Config>,
}

impl App {
    pub fn new() -> color_eyre::Result<Self> {
        let config = Config::load()?;
        let screen = match &config {
            Some(_) => Screen::Images,
            None => Screen::Setup,
        };
        Ok(App { screen, config })
    }
}
