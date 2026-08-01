//! Dispatch of the lifecycle messages: frames, polls, reloads, shutdown.

use iced::Task;
use log::{info, warn};

use super::super::super::{
    shutdown,
    state::{App, Message}
};

impl App {
    /// Handles the messages this module owns.
    pub(crate) fn update_lifecycle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Frame(now) => self.on_frame(now),
            Message::BusFlushed(outcome) => {
                if outcome.is_empty() {
                    Task::none()
                } else {
                    let tasks: Vec<_> = outcome
                        .into_events()
                        .into_iter()
                        .map(Self::message_from_bus_event)
                        .map(|msg| self.update(msg))
                        .collect();

                    Task::batch(tasks)
                }
            }
            Message::PollAtRest => {
                let now = std::time::Instant::now();

                for module in self.attention.due_at_rest(now) {
                    self.poll_module(&module);
                }

                Task::none()
            }
            Message::PollAttended => {
                let now = std::time::Instant::now();

                if let Some(module) = self.attention.due_attended(now) {
                    self.poll_module(&module);
                }

                Task::none()
            }
            Message::ConfigChanged(update) => self.on_config_changed(update),
            Message::Shutdown(signal) => {
                info!("shutting down on {signal:?}, removing every surface");
                shutdown::exit_backstop();

                self.outputs
                    .destroy_all()
                    .chain(Task::done(Message::SurfacesRemoved))
            }
            Message::SurfacesRemoved => shutdown::exit_now(),
            Message::ConfigDegraded(degradation) => {
                warn!("Configuration degradation reported: {}", degradation.reason);
                Task::none()
            }
            _ => Task::none()
        }
    }
}
