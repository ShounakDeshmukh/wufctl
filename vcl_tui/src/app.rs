use crate::config::Config;
use crate::state::{App, ImagesUiState, Screen, SetupState, SetupUiState};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
const VCL_ENDPOINT: &str = "https://vcl.ncsu.edu/scheduling/index.php?mode=xmlrpccall";

impl App {
    pub fn new() -> color_eyre::Result<Self> {
        let config = Config::load()?;
        let screen = match &config {
            Some(_) => Screen::Images,
            None => Screen::Setup,
        };
        let client = config
            .as_ref()
            .map(|cfg| vcl_lib::VclClient::new(VCL_ENDPOINT, cfg.token().to_string()));
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
            images: ImagesUiState::default(),
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        while !self.exit {
            if self.screen == Screen::Images && !self.images.loaded {
                self.load_images()?;
            }
            terminal.draw(|f| crate::ui::draw(self, f))?;
            self.handle_events(terminal)?;
        }
        Ok(())
    }

    /// No busy/toast state yet - the list is simply empty until this
    /// resolves, which is fine given the confirmed sub-second live latency.
    fn load_images(&mut self) -> color_eyre::Result<()> {
        let Some(client) = &self.client else {
            return Ok(());
        };
        match self.async_runtime.block_on(crate::vcl::get_images(client)) {
            Ok(images) => {
                self.images
                    .list_state
                    .select(if images.is_empty() { None } else { Some(0) });
                self.images.images = images;
                self.images.error = None;
            }
            Err(err) => self.images.error = Some(format!("{err:#}")),
        }
        self.images.loaded = true;
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
            Screen::Images => self.handle_images_key(key_event),
            _ => {}
        }

        Ok(())
    }

    fn handle_images_key(&mut self, key_event: crossterm::event::KeyEvent) {
        let len = self.images.images.len();
        if len == 0 {
            return;
        }
        let i = self.images.list_state.selected().unwrap_or(0);
        match key_event.code {
            KeyCode::Up => {
                self.images.list_state.select(Some(i.saturating_sub(1)));
                self.images.message = None;
            }
            KeyCode::Down => {
                self.images.list_state.select(Some((i + 1).min(len - 1)));
                self.images.message = None;
            }
            KeyCode::Char('n') | KeyCode::Enter => {
                let img = &self.images.images[i];
                if !img.is_reservable() {
                    self.open_avd_guide();
                }
                // reservable branch (New Reservation popup) is step 4, not yet wired.
            }
            _ => {}
        }
    }

    /// Debounced so spamming `n` can't spawn a browser per keypress.
    fn open_avd_guide(&mut self) {
        const COOLDOWN: std::time::Duration = std::time::Duration::from_secs(2);
        if self
            .images
            .last_avd_open
            .is_some_and(|t| t.elapsed() < COOLDOWN)
        {
            return;
        }
        self.images.last_avd_open = Some(std::time::Instant::now());
        self.images.message = Some(
            match crate::connect_action::open_url(crate::connect_action::AVD_GUIDE_URL) {
                Ok(()) => Ok("Opened AVD guide in your browser.".to_string()),
                Err(err) => Err(format!("Couldn't open browser: {err}")),
            },
        );
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
