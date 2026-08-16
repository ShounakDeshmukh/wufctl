use std::sync::{Arc, mpsc};
use std::thread;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::DefaultTerminal;

use crate::connect_action;
use crate::state::{App, PendingOp, Popup};
use crate::vcl::{self, ConnectData, ConnectDataResult};

/// The xRDP method's remote port, e.g. "3389" from a "TCP:3389:3389" `connectports` entry.
fn rdp_port(data: &ConnectData) -> Option<&str> {
    data.connect_methods
        .iter()
        .find(|m| m.description.to_lowercase().contains("rdp"))
        .and_then(|m| m.connectports.first())
        .and_then(|p| p.rsplit(':').next())
}

impl App {
    pub(super) fn handle_connect_key(
        &mut self,
        key_event: KeyEvent,
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        if key_event.code == KeyCode::Esc {
            if self.pending.is_some() {
                self.show_toast(Err("Still working - hang on...".to_string()));
                return Ok(());
            }
            self.popup = Popup::None;
            return Ok(());
        }

        if key_event.code == KeyCode::Enter {
            let Some(data) = self.connect.data.clone() else {
                return Ok(());
            };
            match connect_action::connect_ssh(&data.user, &data.server_ip, &data.connect_port) {
                Ok(_status) => {
                    self.popup = Popup::None;
                    self.show_toast(Ok("wufctl resumed - ssh session ended.".to_string()));
                }
                Err(err) => {
                    self.connect.error = Some(connect_action::describe_ssh_error(&err));
                }
            }
            terminal.clear()?;
        }

        if key_event.code == KeyCode::Char('r') {
            let Some(data) = &self.connect.data else {
                return Ok(());
            };
            let Some(port) = rdp_port(data) else {
                return Ok(());
            };
            let result = connect_action::save_rdp_file(
                &self.connect.image_name,
                &data.server_ip,
                port,
                &data.user,
            );
            match result {
                Ok(path) => {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("reservation.rdp");
                    self.show_toast(Ok(format!("RDP file saved: {name}")));
                }
                Err(err) => self.connect.error = Some(format!("Couldn't save RDP file: {err}")),
            }
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
