use crate::config::Config;
use crate::state::{
    App, DURATION_PRESETS, EXTEND_PRESETS, ExtendFormState, ExtendRow, FormRow, ImagesUiState,
    NewReservationFormState, PendingOp, Popup, Reservation, ReservationsUiState, Screen,
    SetupState, SetupUiState, Toast,
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
const VCL_ENDPOINT: &str = "https://vcl.ncsu.edu/scheduling/index.php?mode=xmlrpccall";
/// Matches the VCL web UI's own poll cadence; also the `r` refresh debounce.
const RESERVATIONS_POLL_INTERVAL: Duration = Duration::from_secs(20);
/// How long `handle_events` waits for input before returning, so `run()`'s
/// loop wakes up periodically to check the reservations poll timer even
/// with no keypress.
const EVENT_POLL_RATE: Duration = Duration::from_millis(250);
const TOAST_DURATION: Duration = Duration::from_secs(3);
/// Session-scoped image-list cache - reopening the picker reuses it rather
/// than refetching every time, unless it's older than this.
const IMAGES_CACHE_INTERVAL: Duration = Duration::from_secs(600);

impl App {
    pub fn new() -> color_eyre::Result<Self> {
        let config = Config::load()?;
        let screen = match &config {
            Some(_) => Screen::Reservations,
            None => Screen::Setup,
        };
        let client = config.as_ref().map(|cfg| {
            Arc::new(vcl_lib::VclClient::new(
                VCL_ENDPOINT,
                cfg.token().to_string(),
            ))
        });
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(App {
            screen,
            popup: Popup::None,
            config,
            exit: false,
            async_runtime: Arc::new(rt),
            client,
            setup: SetupUiState::default(),
            images: ImagesUiState::default(),
            reservations: ReservationsUiState::default(),
            new_reservation: NewReservationFormState::default(),
            extend: ExtendFormState::default(),
            toast: None,
            pending: None,
            throbber_state: throbber_widgets_tui::ThrobberState::default(),
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
            self.poll_pending();
            // Only ever one background call in flight app-wide - don't
            // start another while one's still running.
            if self.pending.is_none() {
                if self.popup == Popup::ImagePicker && self.images_cache_stale() {
                    self.trigger_load_images();
                }
                if self.screen == Screen::Reservations && self.should_auto_poll_reservations() {
                    self.trigger_load_reservations();
                }
            }
            terminal.draw(|f| crate::ui::draw(self, f))?;
            self.handle_events()?;
        }
        Ok(())
    }

    /// Checks the in-flight background call (if any) without blocking;
    /// applies its result and clears `pending` once it lands.
    fn poll_pending(&mut self) {
        let Some(op) = self.pending.take() else {
            return;
        };
        match op {
            PendingOp::SetupTest { token, rx } => match rx.try_recv() {
                Ok(result) => self.apply_setup_test(token, result),
                Err(mpsc::TryRecvError::Empty) => {
                    self.pending = Some(PendingOp::SetupTest { token, rx });
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.setup.state =
                        SetupState::Error("Background sign-in task failed unexpectedly.".into());
                }
            },
            PendingOp::LoadImages(rx) => match rx.try_recv() {
                Ok(result) => self.apply_load_images(result),
                Err(mpsc::TryRecvError::Empty) => self.pending = Some(PendingOp::LoadImages(rx)),
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.images.error = Some("Background load task failed unexpectedly.".into());
                    self.images.last_loaded = Some(Instant::now());
                }
            },
            PendingOp::LoadReservations(rx) => match rx.try_recv() {
                Ok(result) => self.apply_load_reservations(result),
                Err(mpsc::TryRecvError::Empty) => {
                    self.pending = Some(PendingOp::LoadReservations(rx));
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.reservations.error =
                        Some("Background load task failed unexpectedly.".into());
                }
            },
            PendingOp::EndReservation { id, index, rx } => match rx.try_recv() {
                Ok(result) => self.apply_end_reservation(id, index, result),
                Err(mpsc::TryRecvError::Empty) => {
                    self.pending = Some(PendingOp::EndReservation { id, index, rx });
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.reservations.message =
                        Some(Err("Background task failed unexpectedly.".into()));
                }
            },
            PendingOp::AddRequest(rx) => match rx.try_recv() {
                Ok(result) => self.apply_add_request(result),
                Err(mpsc::TryRecvError::Empty) => self.pending = Some(PendingOp::AddRequest(rx)),
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.new_reservation.message =
                        Some("Background task failed unexpectedly.".into());
                }
            },
            PendingOp::ExtendRequest { id, rx } => match rx.try_recv() {
                Ok(result) => self.apply_extend_request(id, result),
                Err(mpsc::TryRecvError::Empty) => {
                    self.pending = Some(PendingOp::ExtendRequest { id, rx });
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.extend.message = Some("Background task failed unexpectedly.".into());
                }
            },
        }
    }

    /// `true` on first open, or once `IMAGES_CACHE_INTERVAL` has passed
    /// since the last successful load.
    fn images_cache_stale(&self) -> bool {
        self.images
            .last_loaded
            .is_none_or(|t| t.elapsed() >= IMAGES_CACHE_INTERVAL)
    }

    fn trigger_load_images(&mut self) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let rt = Arc::clone(&self.async_runtime);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = rt.block_on(crate::vcl::get_images(&client));
            let _ = tx.send(result);
        });
        self.pending = Some(PendingOp::LoadImages(rx));
    }

    fn apply_load_images(&mut self, result: color_eyre::Result<Vec<crate::vcl::Image>>) {
        match result {
            Ok(images) => {
                self.images
                    .list_state
                    .select(if images.is_empty() { None } else { Some(0) });
                self.images.images = images;
                self.images.error = None;
            }
            Err(err) => self.images.error = Some(format!("{err:#}")),
        }
        self.images.last_loaded = Some(Instant::now());
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
    /// `get_request_ids()`'s response) then one status per id, all inside
    /// the background thread's single `block_on` call.
    fn trigger_load_reservations(&mut self) {
        self.reservations.last_poll = Some(Instant::now());
        let Some(client) = self.client.clone() else {
            return;
        };
        let rt = Arc::clone(&self.async_runtime);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = rt.block_on(async {
                let entries = crate::vcl::get_request_ids(&client).await?;
                let mut reservations = Vec::with_capacity(entries.len());
                for entry in entries {
                    let status = crate::vcl::get_request_status(&client, entry.requestid).await?;
                    reservations.push(Reservation {
                        id: entry.requestid,
                        image_name: entry.imagename,
                        status,
                        start: entry.start,
                        end: entry.end,
                    });
                }
                Ok(reservations)
            });
            let _ = tx.send(result);
        });
        self.pending = Some(PendingOp::LoadReservations(rx));
    }

    fn apply_load_reservations(&mut self, result: color_eyre::Result<Vec<Reservation>>) {
        match result {
            Ok(reservations) => {
                // Preserve the current selection across a refresh instead
                // of jumping back to the top every 20s.
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
            }
            Err(err) => self.reservations.error = Some(format!("{err:#}")),
        }
    }

    /// Times out after `EVENT_POLL_RATE` rather than blocking forever, so
    /// `run()`'s loop wakes up periodically to check the reservations poll
    /// timer and the pending background call even while the user isn't
    /// pressing anything.
    pub fn handle_events(&mut self) -> color_eyre::Result<()> {
        if !event::poll(EVENT_POLL_RATE)? {
            return Ok(());
        }
        match event::read()? {
            // crossterm also emits key release/repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)?
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
            Popup::ExtendForm { id } => return self.handle_extend_key(key_event, id),
        }

        match self.screen {
            Screen::Setup => self.handle_setup_key(key_event)?,
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
                self.trigger_load_reservations();
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
            // Drop any stale search each time; the image list itself is
            // cached (see `images_cache_stale`), not force-refetched.
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
            KeyCode::Char('x') if status == "ready" => {
                if self.pending.is_some() {
                    // e.g. an auto-poll refresh is mid-flight - don't
                    // stomp it by starting a second background op.
                    self.show_toast(Err("Still working - hang on...".to_string()));
                } else {
                    self.trigger_end_reservation(i);
                }
            }
            KeyCode::Char('e') if status == "ready" => {
                self.popup = Popup::ExtendForm {
                    id: self.reservations.reservations[i].id,
                };
                self.extend = ExtendFormState::default();
            }
            _ => {}
        }
        Ok(())
    }

    fn trigger_end_reservation(&mut self, index: usize) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let id = self.reservations.reservations[index].id;
        let rt = Arc::clone(&self.async_runtime);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = rt.block_on(crate::vcl::end_request(&client, id));
            let _ = tx.send(result);
        });
        self.pending = Some(PendingOp::EndReservation { id, index, rx });
    }

    fn apply_end_reservation(
        &mut self,
        id: i64,
        index: usize,
        result: color_eyre::Result<crate::vcl::ActionResult>,
    ) {
        self.reservations.message = Some(match result {
            Ok(crate::vcl::ActionResult::Success { .. }) => {
                self.reservations.reservations.remove(index);
                self.reservations
                    .list_state
                    .select(if self.reservations.reservations.is_empty() {
                        None
                    } else {
                        Some(index.min(self.reservations.reservations.len() - 1))
                    });
                Ok(format!("Reservation #{id} ended."))
            }
            Ok(crate::vcl::ActionResult::Error { errormsg, .. }) => Err(errormsg),
            Err(err) => Err(format!("{err:#}")),
        });
    }

    fn handle_extend_key(
        &mut self,
        key_event: crossterm::event::KeyEvent,
        id: i64,
    ) -> color_eyre::Result<()> {
        if key_event.code == KeyCode::Esc {
            // Only ExtendRequest could be pending while this popup is
            // open - block navigating away mid-submit, same reasoning as
            // the New Reservation form's Esc guard.
            if self.pending.is_some() {
                self.show_toast(Err("Still working - hang on...".to_string()));
                return Ok(());
            }
            self.popup = Popup::None;
            self.extend = ExtendFormState::default();
            return Ok(());
        }

        match key_event.code {
            KeyCode::Up => self.extend.focus = ExtendRow::Duration,
            KeyCode::Down => self.extend.focus = ExtendRow::Confirm,
            KeyCode::Left => self.adjust_extend_duration(false),
            KeyCode::Right => self.adjust_extend_duration(true),
            KeyCode::Enter if self.extend.focus == ExtendRow::Confirm => {
                self.trigger_extend_request(id);
            }
            _ => {}
        }
        Ok(())
    }

    fn adjust_extend_duration(&mut self, forward: bool) {
        let len = EXTEND_PRESETS.len();
        self.extend.duration_idx = if forward {
            (self.extend.duration_idx + 1) % len
        } else {
            (self.extend.duration_idx + len - 1) % len
        };
    }

    fn trigger_extend_request(&mut self, id: i64) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let Some(&(minutes, _)) = EXTEND_PRESETS.get(self.extend.duration_idx) else {
            return;
        };
        let rt = Arc::clone(&self.async_runtime);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = rt.block_on(crate::vcl::extend_request(&client, id, minutes));
            let _ = tx.send(result);
        });
        self.pending = Some(PendingOp::ExtendRequest { id, rx });
    }

    fn apply_extend_request(
        &mut self,
        id: i64,
        result: color_eyre::Result<crate::vcl::ActionResult>,
    ) {
        match result {
            Ok(crate::vcl::ActionResult::Success { .. }) => {
                self.popup = Popup::None;
                self.extend = ExtendFormState::default();
                // Server is the source of truth - force a fresh fetch,
                // same reasoning as a successful add_request.
                self.reservations.last_poll = None;
                self.show_toast(Ok(format!("Reservation #{id} extended.")));
            }
            Ok(crate::vcl::ActionResult::Error { errormsg, .. }) => {
                self.extend.message = Some(errormsg);
            }
            Err(err) => {
                self.extend.message = Some(describe_rejected_connection(
                    &err,
                    "this extension likely exceeds your account's total time limit. \
                     Try a shorter extension.",
                ));
            }
        }
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

    fn handle_new_reservation_key(
        &mut self,
        key_event: crossterm::event::KeyEvent,
        image_idx: usize,
    ) -> color_eyre::Result<()> {
        if key_event.code == KeyCode::Esc {
            // While AddRequest is pending, this popup is the only thing
            // that could be in flight - block navigating away, since
            // apply_add_request unconditionally closes the popup on
            // success and would otherwise stomp wherever Esc had gone.
            if self.pending.is_some() {
                self.show_toast(Err("Still working - hang on...".to_string()));
                return Ok(());
            }
            // Back one step, to the picker - not all the way to Reservations.
            self.popup = Popup::ImagePicker;
            self.new_reservation = NewReservationFormState::default();
            return Ok(());
        }

        let rows = self.new_reservation.visible_rows();
        let pos = rows
            .iter()
            .position(|&r| r == self.new_reservation.focus)
            .unwrap_or(0);

        match key_event.code {
            KeyCode::Up => self.new_reservation.focus = rows[pos.saturating_sub(1)],
            KeyCode::Down => self.new_reservation.focus = rows[(pos + 1).min(rows.len() - 1)],
            KeyCode::Left => self.adjust_new_reservation_field(false),
            KeyCode::Right => self.adjust_new_reservation_field(true),
            KeyCode::Enter if self.new_reservation.focus == FormRow::Create => {
                self.trigger_submit_new_reservation(image_idx);
            }
            KeyCode::Char(c)
                if self.new_reservation.focus == FormRow::CustomMinutes && c.is_ascii_digit() =>
            {
                let idx = crate::utils::char_to_byte_index(
                    &self.new_reservation.custom_minutes,
                    self.new_reservation.custom_cursor,
                );
                self.new_reservation.custom_minutes.insert(idx, c);
                self.new_reservation.custom_cursor += 1;
            }
            KeyCode::Backspace
                if self.new_reservation.focus == FormRow::CustomMinutes
                    && self.new_reservation.custom_cursor > 0 =>
            {
                let idx = crate::utils::char_to_byte_index(
                    &self.new_reservation.custom_minutes,
                    self.new_reservation.custom_cursor - 1,
                );
                self.new_reservation.custom_minutes.remove(idx);
                self.new_reservation.custom_cursor -= 1;
            }
            _ => {}
        }
        Ok(())
    }

    /// `Left`/`Right` on the focused row: cycles that row's value, except
    /// on `CustomMinutes` where it moves the text cursor instead.
    fn adjust_new_reservation_field(&mut self, forward: bool) {
        use crate::state::{AmPm, StartChoice};
        let f = &mut self.new_reservation;
        match f.focus {
            FormRow::Start => {
                f.start = if f.start == StartChoice::Now {
                    StartChoice::Later
                } else {
                    StartChoice::Now
                };
            }
            FormRow::Day => {
                f.day_offset = if forward {
                    (f.day_offset + 1).min(7)
                } else {
                    f.day_offset.saturating_sub(1)
                };
            }
            FormRow::Hour => {
                f.hour = if forward {
                    if f.hour == 12 { 1 } else { f.hour + 1 }
                } else if f.hour == 1 {
                    12
                } else {
                    f.hour - 1
                };
            }
            FormRow::Minute => {
                let len = crate::state::MINUTE_STEPS.len() as u8;
                f.minute_idx = if forward {
                    (f.minute_idx + 1) % len
                } else {
                    (f.minute_idx + len - 1) % len
                };
            }
            FormRow::AmPm => {
                f.am_pm = if f.am_pm == AmPm::Am {
                    AmPm::Pm
                } else {
                    AmPm::Am
                };
            }
            FormRow::Duration => {
                let len = DURATION_PRESETS.len() + 1; // +1 for "Custom"
                f.duration_idx = if forward {
                    (f.duration_idx + 1) % len
                } else {
                    (f.duration_idx + len - 1) % len
                };
            }
            FormRow::CustomMinutes => {
                f.custom_cursor = if forward {
                    (f.custom_cursor + 1).min(f.custom_minutes.chars().count())
                } else {
                    f.custom_cursor.saturating_sub(1)
                };
            }
            FormRow::Create => {}
        }
    }

    fn trigger_submit_new_reservation(&mut self, image_idx: usize) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let Some(length) = self.new_reservation.duration_minutes() else {
            self.new_reservation.message =
                Some("Enter a valid custom duration in minutes.".to_string());
            return;
        };
        let Some(start) = self.new_reservation.start_value() else {
            self.new_reservation.message =
                Some("Couldn't compute that start time - try a different one.".to_string());
            return;
        };
        let Some(image) = self.images.images.get(image_idx) else {
            self.popup = Popup::None;
            return;
        };
        let image_id = image.id;
        let rt = Arc::clone(&self.async_runtime);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = rt.block_on(crate::vcl::add_request(&client, image_id, &start, length));
            let _ = tx.send(result);
        });
        self.pending = Some(PendingOp::AddRequest(rx));
    }

    fn apply_add_request(&mut self, result: color_eyre::Result<crate::vcl::ActionResult>) {
        match result {
            Ok(crate::vcl::ActionResult::Success { requestid }) => {
                self.popup = Popup::None;
                self.new_reservation = NewReservationFormState::default();
                // Server is the source of truth - force a fresh fetch
                // (rather than inserting a locally-guessed row) next
                // time `run()`'s loop checks the poll timer.
                self.reservations.last_poll = None;
                let message = match requestid {
                    Some(id) => format!("Reservation #{id} created."),
                    None => "Reservation created.".to_string(),
                };
                self.show_toast(Ok(message));
            }
            Ok(crate::vcl::ActionResult::Error { errormsg, .. }) => {
                self.new_reservation.message = Some(errormsg);
            }
            Err(err) => {
                self.new_reservation.message = Some(describe_rejected_connection(
                    &err,
                    "the duration or start time is likely not allowed for this image/account. \
                     Try a shorter duration or a different start.",
                ));
            }
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
            KeyCode::Enter if self.pending.is_none() => self.trigger_submit_token(),
            _ => {}
        }

        Ok(())
    }

    fn trigger_submit_token(&mut self) {
        let token = self.setup.input.trim().to_string();
        if token.is_empty() {
            self.setup.state = SetupState::Error("Token can't be empty.".to_string());
            return;
        }

        let rt = Arc::clone(&self.async_runtime);
        let (tx, rx) = mpsc::channel();
        let client_token = token.clone();
        std::thread::spawn(move || {
            let client = vcl_lib::VclClient::new(VCL_ENDPOINT, client_token);
            let result = rt
                .block_on(client.test("vcl_tui"))
                .map(|_| client)
                .map_err(color_eyre::eyre::Report::from);
            let _ = tx.send(result);
        });
        self.setup.state = SetupState::Validating;
        self.pending = Some(PendingOp::SetupTest { token, rx });
    }

    fn apply_setup_test(&mut self, token: String, result: color_eyre::Result<vcl_lib::VclClient>) {
        match result {
            Ok(client) => {
                let config = Config::new(token);
                match config.save() {
                    Ok(()) => {
                        self.client = Some(Arc::new(client));
                        self.config = Some(config);
                        self.screen = Screen::Reservations;
                        self.setup = SetupUiState::default();
                    }
                    Err(err) => self.setup.state = SetupState::Error(format!("{err:#}")),
                }
            }
            Err(err) => self.setup.state = SetupState::Error(format!("{err}")),
        }
    }
}

/// The server drops the connection outright - no XML-RPC fault, no HTTP
/// error status - when it rejects certain requests outright; confirmed
/// live for `add_request` with an out-of-range custom duration. There's no
/// structured reason in that case, so a raw connection-failure string is
/// translated into an actionable guess (`hint`) rather than shown as-is.
fn describe_rejected_connection(err: &color_eyre::eyre::Report, hint: &str) -> String {
    if let Some(vcl_lib::VclError::HttpError(_)) = err.downcast_ref::<vcl_lib::VclError>() {
        return format!("Server rejected the request - {hint}");
    }
    format!("{err:#}")
}
