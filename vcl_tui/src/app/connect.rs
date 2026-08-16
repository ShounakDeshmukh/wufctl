use std::sync::{Arc, mpsc};
use std::thread;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::state::{App, PendingOp, Popup};
use crate::vcl::{self, ConnectDataResult};

impl App {
    pub(super) fn handle_connect_key(&mut self, key_event: KeyEvent) -> Result<()> {
        if key_event.code == KeyCode::Esc {
            if self.pending.is_some() {
                self.show_toast(Err("Still working - hang on...".to_string()));
                return Ok(());
            }
            self.popup = Popup::None;
        }
        Ok(())
    }

    pub(super) fn trigger_load_connect_data(&mut self, id: i64) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let rt = Arc::clone(&self.async_runtime);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = rt.block_on(async {
                let ip = vcl::get_ip(&client).await?;
                vcl::get_request_connect_data(&client, id, &ip).await
            });
            let _ = tx.send(result);
        });
        self.pending = Some(PendingOp::LoadConnectData(rx));
    }

    pub(super) fn apply_load_connect_data(&mut self, result: Result<ConnectDataResult>) {
        match result {
            Ok(ConnectDataResult::Ready(data)) => self.connect.data = Some(data),
            Ok(ConnectDataResult::NotReady) => {
                self.connect.error = Some("Not ready yet - try again in a moment.".to_string());
            }
            Err(err) => self.connect.error = Some(format!("{err:#}")),
        }
    }
}
