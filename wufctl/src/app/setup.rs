use std::sync::{Arc, mpsc};
use std::thread;

use arboard::Clipboard;
use color_eyre::Result;
use color_eyre::eyre::Report;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use vcl_lib::VclClient;

use crate::config::Config;
use crate::state::{App, PendingOp, Screen, SetupState, SetupUiState};
use crate::utils;

use super::VCL_ENDPOINT;

impl App {
    pub(super) fn handle_setup_key(&mut self, key_event: KeyEvent) -> Result<()> {
        if key_event.modifiers.contains(KeyModifiers::CONTROL)
            && key_event.code == KeyCode::Char('r')
        {
            self.setup.masked = !self.setup.masked;
            return Ok(());
        }

        // Most Linux terminals only bind Ctrl+Shift+V, so fall back to reading the clipboard directly.
        if key_event.modifiers.contains(KeyModifiers::CONTROL)
            && key_event.code == KeyCode::Char('v')
        {
            if let Ok(text) = Clipboard::new().and_then(|mut cb| cb.get_text()) {
                self.handle_paste(text);
            }
            return Ok(());
        }

        match key_event.code {
            KeyCode::Char(c) if !key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                let idx = utils::char_to_byte_index(&self.setup.input, self.setup.cursor);
                self.setup.input.insert(idx, c);
                self.setup.cursor += 1;
            }
            KeyCode::Backspace => {
                if self.setup.cursor > 0 {
                    let idx = utils::char_to_byte_index(&self.setup.input, self.setup.cursor - 1);
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

    /// Strips control chars (e.g. a trailing newline from the clipboard).
    pub(super) fn handle_paste(&mut self, text: String) {
        if self.screen != Screen::Setup || matches!(self.setup.state, SetupState::Validating) {
            return;
        }
        let clean: String = text.chars().filter(|c| !c.is_control()).collect();
        let idx = utils::char_to_byte_index(&self.setup.input, self.setup.cursor);
        self.setup.input.insert_str(idx, &clean);
        self.setup.cursor += clean.chars().count();
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
        thread::spawn(move || {
            let client = VclClient::new(VCL_ENDPOINT, client_token);
            let result = rt
                .block_on(client.test("wufctl"))
                .map(|_| client)
                .map_err(Report::from);
            let _ = tx.send(result);
        });
        self.setup.state = SetupState::Validating;
        self.pending = Some(PendingOp::SetupTest { token, rx });
    }

    pub(super) fn apply_setup_test(&mut self, token: String, result: Result<VclClient>) {
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
