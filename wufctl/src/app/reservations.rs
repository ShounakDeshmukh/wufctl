use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::state::{
    App, ConnectUiState, ExtendFormState, PendingOp, Popup, Reservation, Screen, SetupState,
    SetupUiState,
};
use crate::vcl::{self, ActionResult};
use vcl_lib::VclError;

const RESERVATIONS_POLL_INTERVAL: Duration = Duration::from_secs(20);

impl App {
    /// True on first load, or once the poll interval has passed (the `r` debounce).
    fn reservations_poll_due(&self) -> bool {
        self.reservations
            .last_poll
            .is_none_or(|t| t.elapsed() >= RESERVATIONS_POLL_INTERVAL)
    }

    /// Auto-polls while something's still loading, then stops once everything settles.
    pub(super) fn should_auto_poll_reservations(&self) -> bool {
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

    /// Fetches the list, then one status per id, inside a single `block_on` call.
    pub(super) fn trigger_load_reservations(&mut self) {
        self.reservations.last_poll = Some(Instant::now());
        let Some(client) = self.client.clone() else {
            return;
        };
        let rt = Arc::clone(&self.async_runtime);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = rt.block_on(async {
                let entries = vcl::get_request_ids(&client).await?;
                let mut reservations = Vec::with_capacity(entries.len());
                for entry in entries {
                    let status = vcl::get_request_status(&client, entry.requestid).await?;
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

    pub(super) fn apply_load_reservations(&mut self, result: Result<Vec<Reservation>>) {
        match result {
            Ok(reservations) => {
                // Preserve the selection across a refresh instead of jumping to the top.
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
            Err(err) if is_auth_error(&err) => {
                self.screen = Screen::Setup;
                self.setup = SetupUiState {
                    state: SetupState::Error("Token rejected - sign in again.".to_string()),
                    ..SetupUiState::default()
                };
            }
            Err(err) => self.reservations.error = Some(format!("{err:#}")),
        }
    }

    pub(super) fn handle_reservations_key(&mut self, key_event: KeyEvent) -> Result<()> {
        if key_event.code == KeyCode::Char('q') {
            self.exit = true;
            return Ok(());
        }

        if key_event.code == KeyCode::Char('r') {
            if self.reject_if_busy() {
                return Ok(());
            }
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
            // Drop any stale search; the image list itself stays cached.
            self.images.search.clear();
            self.images.search_cursor = 0;
            self.images.searching = false;
            return Ok(());
        }

        if key_event.code == KeyCode::Char('t') {
            self.popup = Popup::ConfirmChangeToken;
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
            KeyCode::Char('x') if status != "loading" => {
                self.popup = Popup::ConfirmEnd { index: i };
            }
            KeyCode::Char('e') if status == "ready" => {
                self.popup = Popup::ExtendForm {
                    id: self.reservations.reservations[i].id,
                };
                self.extend = ExtendFormState::default();
            }
            KeyCode::Char('c') if status == "ready" => {
                if self.reject_if_busy() {
                    return Ok(());
                }
                let id = self.reservations.reservations[i].id;
                let image_name = self.reservations.reservations[i].image_name.clone();
                self.popup = Popup::Connect { id };
                self.connect = ConnectUiState {
                    image_name,
                    ..ConnectUiState::default()
                };
                self.trigger_load_connect_data(id);
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn handle_confirm_end_key(
        &mut self,
        key_event: KeyEvent,
        index: usize,
    ) -> Result<()> {
        if key_event.code == KeyCode::Esc {
            self.popup = Popup::None;
            return Ok(());
        }
        if key_event.code == KeyCode::Enter {
            if self.reject_if_busy() {
                return Ok(());
            }
            self.popup = Popup::None;
            self.trigger_end_reservation(index);
        }
        Ok(())
    }

    pub(super) fn handle_confirm_change_token_key(&mut self, key_event: KeyEvent) -> Result<()> {
        if key_event.code == KeyCode::Esc {
            self.popup = Popup::None;
            return Ok(());
        }
        if key_event.code == KeyCode::Enter {
            self.popup = Popup::None;
            self.screen = Screen::Setup;
            self.setup = SetupUiState::default();
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
        thread::spawn(move || {
            let result = rt.block_on(vcl::end_request(&client, id));
            let _ = tx.send(result);
        });
        self.pending = Some(PendingOp::EndReservation { id, index, rx });
    }

    pub(super) fn apply_end_reservation(
        &mut self,
        id: i64,
        index: usize,
        result: Result<ActionResult>,
    ) {
        self.reservations.message = Some(match result {
            Ok(ActionResult::Success { .. }) => {
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
            Ok(ActionResult::Error { errormsg, .. }) => Err(errormsg),
            Err(err) => Err(format!("{err:#}")),
        });
    }
}

/// Confirmed live: a rejected token comes back as an XML-RPC fault ("Fault [3] Access denied"),
/// not an HTTP 401/403.
fn is_auth_error(err: &color_eyre::eyre::Report) -> bool {
    match err.downcast_ref::<VclError>() {
        Some(VclError::ApiError(msg)) => msg.to_lowercase().contains("access denied"),
        _ => false,
    }
}
