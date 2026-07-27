//! Compositor output hotplug handling.

use iced::{Task, event::wayland::OutputEvent};
use log::info;

use super::super::state::{App, Message};

impl App {
    /// Handles the messages this module owns.
    pub(super) fn update_outputs(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OutputEvent((event, wl_output)) => match event {
                OutputEvent::Created(info) => {
                    info!("Output created: {info:?}");
                    let name = info
                        .as_ref()
                        .and_then(|info| info.name.as_deref())
                        .unwrap_or("");

                    self.outputs.add(
                        self.config.appearance.style,
                        &self.config.outputs,
                        self.config.position,
                        name,
                        wl_output,
                        &self.config
                    )
                }
                OutputEvent::Removed => {
                    info!("Output destroyed");
                    self.outputs.remove(
                        self.config.appearance.style,
                        self.config.position,
                        wl_output,
                        &self.config
                    )
                }
                _ => Task::none()
            },
            _ => Task::none()
        }
    }
}
