use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::connect_action::{self, AVD_GUIDE_URL};
use crate::state::{App, PendingOp, Popup};
use crate::utils;
use crate::vcl::{self, Image};

/// Session-scoped image cache
const IMAGES_CACHE_INTERVAL: Duration = Duration::from_secs(600);

impl App {
    pub(super) fn is_images_cache_stale(&self) -> bool {
        self.images
            .last_loaded
            .is_none_or(|t| t.elapsed() >= IMAGES_CACHE_INTERVAL)
    }

    pub(super) fn trigger_load_images(&mut self) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let rt = Arc::clone(&self.async_runtime);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = rt.block_on(vcl::get_images(&client));
            let _ = tx.send(result);
        });
        self.pending = Some(PendingOp::LoadImages(rx));
    }

    pub(super) fn apply_load_images(&mut self, result: Result<Vec<Image>>) {
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

    pub(super) fn handle_image_picker_key(&mut self, key_event: KeyEvent) -> Result<()> {
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

    /// Vim-style `/` search: `Enter` keeps the filter, `Esc` clears it.
    fn handle_image_search_key(&mut self, key_event: KeyEvent) -> Result<()> {
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
                let idx = utils::char_to_byte_index(&self.images.search, self.images.search_cursor);
                self.images.search.insert(idx, c);
                self.images.search_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.images.search_cursor > 0 {
                    let idx = utils::char_to_byte_index(
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

    /// Debounced so spamming `n` can't spawn a browser per keypress.
    fn open_avd_guide(&mut self) {
        const COOLDOWN: Duration = Duration::from_secs(2);
        if self
            .images
            .last_avd_open
            .is_some_and(|t| t.elapsed() < COOLDOWN)
        {
            return;
        }
        self.images.last_avd_open = Some(Instant::now());
        self.images.message = Some(match connect_action::open_url(AVD_GUIDE_URL) {
            Ok(()) => Ok("Opened AVD guide in your browser.".to_string()),
            Err(err) => Err(format!("Couldn't open browser: {err}")),
        });
    }
}
