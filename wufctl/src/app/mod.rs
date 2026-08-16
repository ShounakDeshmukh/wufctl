mod connect;
mod extend;
mod images;
mod new_reservation;
mod reservations;
mod setup;

use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use color_eyre::Result;
use color_eyre::eyre::Report;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use throbber_widgets_tui::ThrobberState;
use tokio::runtime::Builder;
use vcl_lib::{VclClient, VclError};

use crate::config::Config;
use crate::state::{
    App, ConnectUiState, ExtendFormState, ImagesUiState, NewReservationFormState, PendingOp, Popup,
    ReservationsUiState, Screen, SetupState, SetupUiState, Toast,
};
use crate::ui;

const VCL_ENDPOINT: &str = "https://vcl.ncsu.edu/scheduling/index.php?mode=xmlrpccall";
/// So `run()`'s loop keeps checking the poll timer even without a keypress.
const EVENT_POLL_RATE: Duration = Duration::from_millis(250);
const TOAST_DURATION: Duration = Duration::from_secs(3);

impl App {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let screen = match &config {
            Some(_) => Screen::Reservations,
            None => Screen::Setup,
        };
        let client = config
            .as_ref()
            .map(|cfg| Arc::new(VclClient::new(VCL_ENDPOINT, cfg.token().to_string())));
        let rt = Builder::new_current_thread().enable_all().build()?;
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
            connect: ConnectUiState::default(),
            toast: None,
            pending: None,
            throbber_state: ThrobberState::default(),
        })
    }

    pub(super) fn show_toast(&mut self, text: Result<String, String>) {
        self.toast = Some(Toast {
            text,
            expires_at: Instant::now() + TOAST_DURATION,
        });
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            self.poll_pending();
            // Only one background call in flight at a time.
            if self.pending.is_none() {
                if self.popup == Popup::ImagePicker && self.is_images_cache_stale() {
                    self.trigger_load_images();
                }
                if self.screen == Screen::Reservations && self.should_auto_poll_reservations() {
                    self.trigger_load_reservations();
                }
            }
            terminal.draw(|f| ui::draw(self, f))?;
            self.handle_events(terminal)?;
        }
        Ok(())
    }

    /// Checks the in-flight background call, if any, without blocking.
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
            PendingOp::LoadConnectData(rx) => match rx.try_recv() {
                Ok(result) => self.apply_load_connect_data(result),
                Err(mpsc::TryRecvError::Empty) => {
                    self.pending = Some(PendingOp::LoadConnectData(rx));
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.connect.error = Some("Background task failed unexpectedly.".into());
                }
            },
        }
    }

    /// Times out instead of blocking forever, so `run()`'s loop keeps checking timers.
    fn handle_events(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
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

    fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        if key_event.modifiers.contains(KeyModifiers::CONTROL)
            && key_event.code == KeyCode::Char('c')
        {
            self.exit = true;
            return Ok(());
        }

        // A popup swallows all keys, taking priority over the screen underneath.
        match self.popup {
            Popup::None => {}
            Popup::ImagePicker => return self.handle_image_picker_key(key_event),
            Popup::NewReservationForm { image_idx } => {
                return self.handle_new_reservation_key(key_event, image_idx);
            }
            Popup::ExtendForm { id } => return self.handle_extend_key(key_event, id),
            Popup::Connect { .. } => return self.handle_connect_key(key_event, terminal),
        }

        match self.screen {
            Screen::Setup => self.handle_setup_key(key_event)?,
            Screen::Reservations => self.handle_reservations_key(key_event)?,
        }

        Ok(())
    }
}

/// The server just drops the connection on some rejections, so turn that into an actionable guess.
fn describe_rejected_connection(err: &Report, hint: &str) -> String {
    if let Some(VclError::HttpError(_)) = err.downcast_ref::<VclError>() {
        return format!("Server rejected the request - {hint}");
    }
    format!("{err:#}")
}
