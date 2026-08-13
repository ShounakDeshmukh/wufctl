use crate::config::Config;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
const VCL_ENDPOINT: &str = "https://vcl.ncsu.edu/scheduling/index.php?mode=xmlrpccall";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Setup,
    Images,
    Reservations,
    Connect,
}

#[derive(Debug, Default)]
pub enum SetupState {
    #[default]
    Idle,
    Validating,
    Error(String),
}

#[derive(Debug)]
pub struct SetupUiState {
    pub input: String,
    /// Char index (not byte index) of the insertion point within `input`.
    pub cursor: usize,
    /// `true` renders `input` as bullets; toggled with Ctrl+R.
    pub masked: bool,
    pub state: SetupState,
}

impl Default for SetupUiState {
    fn default() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            masked: true,
            state: SetupState::default(),
        }
    }
}
pub struct App {
    pub screen: Screen,
    pub config: Option<Config>,
    pub exit: bool,
    pub async_runtime: tokio::runtime::Runtime,
    pub client: Option<vcl_lib::VclClient>,
    pub setup: SetupUiState,
}

impl App {
    pub fn new() -> color_eyre::Result<Self> {
        let config = Config::load()?;
        let screen = match &config {
            Some(_) => Screen::Images,
            None => Screen::Setup,
        };
        let client = match &config {
            Some(cfg) => Some(vcl_lib::VclClient::new(
                VCL_ENDPOINT,
                cfg.token().to_string(),
            )),
            None => None,
        };
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(App {
            screen,
            config,
            exit: false,
            async_runtime: rt,
            client,
            setup: SetupUiState::default(),
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        while !self.exit {
            terminal.draw(|f| crate::ui::draw(self, f))?;
            self.handle_events(terminal)?;
        }
        Ok(())
    }

    pub fn handle_events(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        match event::read()? {
            // crossterm also emits key release/repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event, terminal)?
            }
            Event::Paste(text) => self.handle_paste(text),
            _ => {}
        };
        Ok(())
    }

    /// Strips control chars (e.g. a trailing newline from the clipboard).
    fn handle_paste(&mut self, text: String) {
        if self.screen != Screen::Setup || matches!(self.setup.state, SetupState::Validating) {
            return;
        }
        let clean: String = text.chars().filter(|c| !c.is_control()).collect();
        let idx = crate::utils::char_to_byte_index(&self.setup.input, self.setup.cursor);
        self.setup.input.insert_str(idx, &clean);
        self.setup.cursor += clean.chars().count();
    }

    pub fn handle_key_event(
        &mut self,
        key_event: crossterm::event::KeyEvent,
        terminal: &mut DefaultTerminal,
    ) -> color_eyre::Result<()> {
        if key_event.modifiers.contains(KeyModifiers::CONTROL)
            && key_event.code == KeyCode::Char('c')
        {
            self.exit = true;
            return Ok(());
        }

        match self.screen {
            Screen::Setup => self.handle_setup_key(key_event, terminal)?,
            _ => {}
        }

        Ok(())
    }

    pub fn handle_setup_key(
        &mut self,
        key_event: crossterm::event::KeyEvent,
        terminal: &mut DefaultTerminal,
    ) -> color_eyre::Result<()> {
        if key_event.modifiers.contains(KeyModifiers::CONTROL)
            && key_event.code == KeyCode::Char('r')
        {
            self.setup.masked = !self.setup.masked;
            return Ok(());
        }

        // Most Linux terminals only bind Ctrl+Shift+V, not Ctrl+V, so read
        // the clipboard directly as a fallback; no clipboard (e.g. headless
        // SSH) just no-ops.
        if key_event.modifiers.contains(KeyModifiers::CONTROL)
            && key_event.code == KeyCode::Char('v')
        {
            if let Ok(text) = arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                self.handle_paste(text);
            }
            return Ok(());
        }

        if matches!(self.setup.state, SetupState::Validating) {
            return Ok(());
        }

        match key_event.code {
            KeyCode::Char(c) if !key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                let idx = crate::utils::char_to_byte_index(&self.setup.input, self.setup.cursor);
                self.setup.input.insert(idx, c);
                self.setup.cursor += 1;
            }
            KeyCode::Backspace => {
                if self.setup.cursor > 0 {
                    let idx =
                        crate::utils::char_to_byte_index(&self.setup.input, self.setup.cursor - 1);
                    self.setup.input.remove(idx);
                    self.setup.cursor -= 1;
                }
            }
            KeyCode::Left => self.setup.cursor = self.setup.cursor.saturating_sub(1),
            KeyCode::Right => {
                self.setup.cursor = (self.setup.cursor + 1).min(self.setup.input.chars().count())
            }
            KeyCode::Enter => self.submit_token(terminal)?,
            _ => {}
        }

        Ok(())
    }

    fn submit_token(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        let token = self.setup.input.trim().to_string();
        if token.is_empty() {
            self.setup.state = SetupState::Error("Token can't be empty.".to_string());
            return Ok(());
        }

        self.setup.state = SetupState::Validating;
        terminal.draw(|f| crate::ui::draw(self, f))?;

        let client = vcl_lib::VclClient::new(VCL_ENDPOINT, token.clone());
        let result = self.async_runtime.block_on(client.test("vcl_tui"));

        match result {
            Ok(_) => {
                let config = Config::new(token);
                match config.save() {
                    Ok(()) => {
                        self.client = Some(client);
                        self.config = Some(config);
                        self.screen = Screen::Images;
                        self.setup = SetupUiState::default();
                    }
                    Err(err) => self.setup.state = SetupState::Error(format!("{err:#}")),
                }
            }
            Err(err) => self.setup.state = SetupState::Error(format!("{err}")),
        }

        Ok(())
    }
}
