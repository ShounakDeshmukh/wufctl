use std::sync::{Arc, mpsc};
use std::thread;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::state::{App, EXTEND_PRESETS, ExtendFormState, ExtendRow, PendingOp, Popup};
use crate::vcl::{self, ActionResult};

use super::describe_rejected_connection;

impl App {
    pub(super) fn handle_extend_key(&mut self, key_event: KeyEvent, id: i64) -> Result<()> {
        if key_event.code == KeyCode::Esc {
            // Block navigating away mid-submit, same as the New Reservation form's Esc guard.
            if self.reject_if_busy() {
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
            KeyCode::Enter if self.extend.focus == ExtendRow::Confirm && !self.reject_if_busy() => {
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
        thread::spawn(move || {
            let result = rt.block_on(vcl::extend_request(&client, id, minutes));
            let _ = tx.send(result);
        });
        self.pending = Some(PendingOp::ExtendRequest { id, rx });
    }

    pub(super) fn apply_extend_request(&mut self, id: i64, result: Result<ActionResult>) {
        match result {
            Ok(ActionResult::Success { .. }) => {
                self.popup = Popup::None;
                self.extend = ExtendFormState::default();
                // Server is the source of truth, so force a fresh fetch.
                self.reservations.last_poll = None;
                self.show_toast(Ok(format!("Reservation #{id} extended.")));
            }
            Ok(ActionResult::Error { errormsg, .. }) => {
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
}
