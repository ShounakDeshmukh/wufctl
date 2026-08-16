use std::sync::{Arc, mpsc};
use std::thread;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::state::{
    AmPm, App, DURATION_PRESETS, FormRow, MINUTE_STEPS, NewReservationFormState, PendingOp, Popup,
    StartChoice,
};
use crate::vcl::{self, ActionResult};

use super::describe_rejected_connection;

impl App {
    pub(super) fn handle_new_reservation_key(
        &mut self,
        key_event: KeyEvent,
        image_idx: usize,
    ) -> Result<()> {
        if key_event.code == KeyCode::Esc {
            // Block navigating away while AddRequest is pending, or success would stomp it.
            if self.reject_if_busy() {
                return Ok(());
            }
            // Back one step, to the picker, not all the way to Reservations.
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
                if !self.reject_if_busy() {
                    self.trigger_submit_new_reservation(image_idx);
                }
            }
            KeyCode::Char(c)
                if self.new_reservation.focus == FormRow::CustomMinutes && c.is_ascii_digit() =>
            {
                self.new_reservation.custom_minutes.insert(c);
            }
            KeyCode::Backspace if self.new_reservation.focus == FormRow::CustomMinutes => {
                self.new_reservation.custom_minutes.backspace();
            }
            _ => {}
        }
        Ok(())
    }

    /// `Left`/`Right` cycles the focused row's value, except `CustomMinutes` moves the cursor.
    fn adjust_new_reservation_field(&mut self, forward: bool) {
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
                let len = MINUTE_STEPS.len() as u8;
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
                if forward {
                    f.custom_minutes.right();
                } else {
                    f.custom_minutes.left();
                }
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
        thread::spawn(move || {
            let result = rt.block_on(vcl::add_request(&client, image_id, &start, length));
            let _ = tx.send(result);
        });
        self.pending = Some(PendingOp::AddRequest(rx));
    }

    pub(super) fn apply_add_request(&mut self, result: Result<ActionResult>) {
        match result {
            Ok(ActionResult::Success { requestid }) => {
                self.popup = Popup::None;
                self.new_reservation = NewReservationFormState::default();
                // Server is the source of truth, so force a fresh fetch next poll.
                self.reservations.last_poll = None;
                let message = match requestid {
                    Some(id) => format!("Reservation #{id} created."),
                    None => "Reservation created.".to_string(),
                };
                self.show_toast(Ok(message));
            }
            Ok(ActionResult::Error { errormsg, .. }) => {
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
}
