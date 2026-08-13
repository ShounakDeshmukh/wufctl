use crate::config::Config;
use crate::state::{
    App, ImagesUiState, Popup, Reservation, ReservationsUiState, Screen, SetupState, SetupUiState,
    Toast,
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use std::time::{Duration, Instant};
const VCL_ENDPOINT: &str = "https://vcl.ncsu.edu/scheduling/index.php?mode=xmlrpccall";
/// Matches the VCL web UI's own poll cadence; also the `r` refresh debounce.
const RESERVATIONS_POLL_INTERVAL: Duration = Duration::from_secs(20);
/// How long `handle_events` waits for input before returning, so `run()`'s
/// loop wakes up periodically to check the reservations poll timer even
/// with no keypress.
const EVENT_POLL_RATE: Duration = Duration::from_millis(250);
const TOAST_DURATION: Duration = Duration::from_secs(3);

impl App {
    pub fn new() -> color_eyre::Result<Self> {
        let config = Config::load()?;
        let screen = match &config {
            Some(_) => Screen::Reservations,
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
            popup: Popup::None,
            config,
            exit: false,
            async_runtime: rt,
            client,
            setup: SetupUiState::default(),
            images: ImagesUiState::default(),
            reservations: ReservationsUiState::default(),
            toast: None,
        })
    }

    fn show_toast(&mut self, text: Result<String, String>) {
        self.toast = Some(Toast {
            text,
            expires_at: Instant::now() + TOAST_DURATION,
        });
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        while !self.exit {
            if self.popup == Popup::ImagePicker && !self.images.loaded {
                self.load_images()?;
            }
            if self.screen == Screen::Reservations && self.should_auto_poll_reservations() {
                self.load_reservations()?;
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

    /// `true` on first load, or once `RESERVATIONS_POLL_INTERVAL` has
    /// passed since the last one - the `r` refresh debounce.
    fn reservations_poll_due(&self) -> bool {
        self.reservations
            .last_poll
            .is_none_or(|t| t.elapsed() >= RESERVATIONS_POLL_INTERVAL)
    }

    /// Auto-poll only runs the initial load plus, while something is still
    /// `loading`, one refresh per `RESERVATIONS_POLL_INTERVAL` - once
    /// everything has settled it stops, and only `r` refreshes from there.
    fn should_auto_poll_reservations(&self) -> bool {
        if self.reservations.last_poll.is_none() {
            return true;
        }
        self.reservations_poll_due()
            && self
                .reservations
                .reservations
                .iter()
                .any(|r| r.status.status == "loading")
    }

    /// Fetches the list (id + image name, both confirmed live in
    /// `get_request_ids()`'s response) then one status per id.
    fn load_reservations(&mut self) -> color_eyre::Result<()> {
        self.reservations.last_poll = Some(std::time::Instant::now());
        let Some(client) = &self.client else {
            return Ok(());
        };
        let entries = match self
            .async_runtime
            .block_on(crate::vcl::get_request_ids(client))
        {
            Ok(entries) => entries,
            Err(err) => {
                self.reservations.error = Some(format!("{err:#}"));
                return Ok(());
            }
        };

        let mut reservations = Vec::with_capacity(entries.len());
        for entry in entries {
            let status = match self
                .async_runtime
                .block_on(crate::vcl::get_request_status(client, entry.requestid))
            {
                Ok(status) => status,
                Err(err) => {
                    self.reservations.error = Some(format!("{err:#}"));
                    return Ok(());
                }
            };
            reservations.push(Reservation {
                id: entry.requestid,
                image_name: entry.imagename,
                status,
            });
        }

        // Preserve the current selection across a refresh instead of
        // jumping back to the top every 20s.
        let selected = self.reservations.list_state.selected().unwrap_or(0);
        self.reservations
            .list_state
            .select(if reservations.is_empty() {
                None
            } else {
                Some(selected.min(reservations.len() - 1))
            });
        self.reservations.reservations = reservations;
        self.reservations.error = None;
        Ok(())
    }

    /// Times out after `EVENT_POLL_RATE` rather than blocking forever, so
    /// `run()`'s loop wakes up periodically to check the reservations poll
    /// timer even while the user isn't pressing anything.
    pub fn handle_events(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        if !event::poll(EVENT_POLL_RATE)? {
            return Ok(());
        }
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

        // A popup swallows all keys - it takes priority over the screen
        // underneath, which keeps rendering but shouldn't react to input.
        match self.popup {
            Popup::None => {}
            Popup::ImagePicker => return self.handle_image_picker_key(key_event),
            Popup::NewReservationForm { image_idx } => {
                return self.handle_new_reservation_key(key_event, image_idx);
            }
        }

        match self.screen {
            Screen::Setup => self.handle_setup_key(key_event, terminal)?,
            Screen::Reservations => self.handle_reservations_key(key_event)?,
        }

        Ok(())
    }

    fn handle_reservations_key(
        &mut self,
        key_event: crossterm::event::KeyEvent,
    ) -> color_eyre::Result<()> {
        if key_event.code == KeyCode::Char('r') {
            if self.reservations_poll_due() {
                self.load_reservations()?;
            } else {
                let last_poll = self
                    .reservations
                    .last_poll
                    .expect("reservations_poll_due() being false implies last_poll is Some");
                let remaining = RESERVATIONS_POLL_INTERVAL - last_poll.elapsed();
                self.show_toast(Err(format!(
                    "Refreshed recently - try again in {}s.",
                    remaining.as_secs() + 1
                )));
            }
            return Ok(());
        }

        if key_event.code == KeyCode::Char('n') {
            self.popup = Popup::ImagePicker;
            // Force a fresh fetch, and drop any stale search, each time.
            self.images.loaded = false;
            self.images.search.clear();
            self.images.search_cursor = 0;
            self.images.searching = false;
            return Ok(());
        }

        let len = self.reservations.reservations.len();
        if len == 0 {
            return Ok(());
        }
        let i = self.reservations.list_state.selected().unwrap_or(0);
        let status = self.reservations.reservations[i].status.status.clone();
        match key_event.code {
            KeyCode::Up => {
                self.reservations
                    .list_state
                    .select(Some(i.saturating_sub(1)));
                self.reservations.message = None;
            }
            KeyCode::Down => {
                self.reservations
                    .list_state
                    .select(Some((i + 1).min(len - 1)));
                self.reservations.message = None;
            }
            KeyCode::Char('x') if status == "ready" => self.end_reservation(i),
            KeyCode::Char('e') if status == "ready" => todo!("extend_request - not built yet"),
            _ => {}
        }
        Ok(())
    }

    fn end_reservation(&mut self, i: usize) {
        let Some(client) = &self.client else { return };
        let id = self.reservations.reservations[i].id;
        let result = self
            .async_runtime
            .block_on(crate::vcl::end_request(client, id));
        self.reservations.message = Some(match result {
            Ok(crate::vcl::ActionResult::Success { .. }) => {
                self.reservations.reservations.remove(i);
                self.reservations
                    .list_state
                    .select(if self.reservations.reservations.is_empty() {
                        None
                    } else {
                        Some(i.min(self.reservations.reservations.len() - 1))
                    });
                Ok(format!("Reservation #{id} ended."))
            }
            Ok(crate::vcl::ActionResult::Error {
                errorcode,
                errormsg,
            }) => Err(format!("[{errorcode}] {errormsg}")),
            Err(err) => Err(format!("{err:#}")),
        });
    }

    fn handle_image_picker_key(
        &mut self,
        key_event: crossterm::event::KeyEvent,
    ) -> color_eyre::Result<()> {
        if self.images.searching {
            return self.handle_image_search_key(key_event);
        }

        if key_event.code == KeyCode::Esc {
            self.popup = Popup::None;
            return Ok(());
        }

        if key_event.code == KeyCode::Char('/') {
            self.images.searching = true;
            self.images.search_cursor = self.images.search.chars().count();
            return Ok(());
        }

        let visible = self.images.visible_indices();
        if visible.is_empty() {
            return Ok(());
        }
        let pos = self
            .images
            .list_state
            .selected()
            .unwrap_or(0)
            .min(visible.len() - 1);
        match key_event.code {
            KeyCode::Up => {
                self.images.list_state.select(Some(pos.saturating_sub(1)));
                self.images.message = None;
            }
            KeyCode::Down => {
                self.images
                    .list_state
                    .select(Some((pos + 1).min(visible.len() - 1)));
                self.images.message = None;
            }
            KeyCode::Char('n') | KeyCode::Enter => {
                let real_idx = visible[pos];
                let img = &self.images.images[real_idx];
                if img.is_reservable() {
                    self.popup = Popup::NewReservationForm {
                        image_idx: real_idx,
                    };
                } else {
                    self.open_avd_guide();
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Vim/less-style `/` search: keys edit the query until `Enter`
    /// (confirm, keep the filter) or `Esc` (cancel, clear the filter).
    fn handle_image_search_key(
        &mut self,
        key_event: crossterm::event::KeyEvent,
    ) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Enter => {
                self.images.searching = false;
                return Ok(());
            }
            KeyCode::Esc => {
                self.images.searching = false;
                self.images.search.clear();
                self.images.search_cursor = 0;
            }
            KeyCode::Left => {
                self.images.search_cursor = self.images.search_cursor.saturating_sub(1);
                return Ok(());
            }
            KeyCode::Right => {
                self.images.search_cursor =
                    (self.images.search_cursor + 1).min(self.images.search.chars().count());
                return Ok(());
            }
            KeyCode::Char(c) if !key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                let idx = crate::utils::char_to_byte_index(
                    &self.images.search,
                    self.images.search_cursor,
                );
                self.images.search.insert(idx, c);
                self.images.search_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.images.search_cursor > 0 {
                    let idx = crate::utils::char_to_byte_index(
                        &self.images.search,
                        self.images.search_cursor - 1,
                    );
                    self.images.search.remove(idx);
                    self.images.search_cursor -= 1;
                }
            }
            _ => return Ok(()),
        }
        // Query changed - reset selection to the top of the new filter.
        let visible = self.images.visible_indices();
        self.images
            .list_state
            .select(if visible.is_empty() { None } else { Some(0) });
        Ok(())
    }

    /// Placeholder - the actual Start/Duration form is step 4.
    fn handle_new_reservation_key(
        &mut self,
        key_event: crossterm::event::KeyEvent,
        _image_idx: usize,
    ) -> color_eyre::Result<()> {
        if key_event.code == KeyCode::Esc {
            // Back one step, to the picker - not all the way to Reservations.
            self.popup = Popup::ImagePicker;
        }
        Ok(())
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
                        self.screen = Screen::Reservations;
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
