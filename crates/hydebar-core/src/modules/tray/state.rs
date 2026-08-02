//! Listener wiring and message handling for the tray module.

use std::sync::Arc;

use log::{debug, error, warn};

use super::super::tray::{TrayCommandFuture, TrayMessage, TrayModule};
use crate::services::{ReadOnlyService, ServiceEvent, tray::TrayCommand};

impl TrayModule {
    /// Folds every open tray submenu, for a window opened afresh.
    pub fn collapse_submenus(&mut self) {
        self.submenus.clear();
    }

    pub(super) fn abort_listener_handles(&mut self) {
        for handle in self.listener_handles.drain(..) {
            handle.abort();
        }
    }

    pub(super) fn spawn_listener(&mut self) {
        let Some(sender) = self.sender.clone() else {
            warn!("tray module missing event sender; skipping listener spawn");
            return;
        };
        let Some(runtime) = self.runtime.clone() else {
            warn!("tray module missing runtime handle; skipping listener spawn");
            return;
        };

        let spawner = Arc::clone(&self.listener_spawner);
        self.listener_handles.push(spawner(sender, runtime));
    }

    pub(super) fn dispatch_command(&self, command_future: TrayCommandFuture) {
        let Some(runtime) = self.runtime.clone() else {
            warn!("tray module missing runtime handle; skipping command dispatch");
            return;
        };
        let Some(sender) = self.sender.clone() else {
            warn!("tray module missing event sender; skipping command dispatch");
            return;
        };

        runtime.spawn(async move {
            let event = command_future.await;
            sender.send(TrayMessage::Event(Box::new(event)));
        });
    }

    pub fn update(&mut self, message: TrayMessage) {
        match message {
            TrayMessage::Event(event) => match *event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                }
                ServiceEvent::Update(data) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(data);
                    }
                }
                ServiceEvent::Error(()) => {
                    error!("Tray service error occurred");
                }
            },
            TrayMessage::ToggleSubmenu(index) => {
                if self.submenus.contains(&index) {
                    self.submenus.retain(|i| i != &index);
                } else {
                    self.submenus.push(index);
                }
            }
            TrayMessage::MenuSelected(name, id) => {
                debug!("Tray menu click: {id}");

                if let Some(command) = (self.command_factory)(
                    self.service.as_ref(),
                    TrayCommand::MenuSelected(name, id)
                ) {
                    self.dispatch_command(command);
                }
            }
        }
    }
}
