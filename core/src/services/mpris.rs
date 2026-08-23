//! MPRIS media player service exposing discovered players to the bar.

use std::ops::Deref;

use commands::execute_player_command;
use iced::{Subscription, Task};
use zbus::Connection;

use super::{ReadOnlyService, Service, ServiceEvent};
use crate::modules::ModuleError;

mod commands;
/// What a player reports about itself.
pub mod data;
mod dbus;
mod ipc;
mod listener;

pub use commands::{MprisPlayerCommand, PlayerCommand};
pub use data::{MprisPlayerData, MprisPlayerEvent, MprisPlayerMetadata, PlaybackStatus};
pub(crate) use listener::{ListenerState, MprisEventPublisher};

/// Service storing the currently discovered MPRIS players and their cached
/// state.
#[derive(Debug, Clone)]
pub struct MprisPlayerService {
    data: Vec<MprisPlayerData>,
    conn: Connection
}

impl Deref for MprisPlayerService {
    type Target = Vec<MprisPlayerData>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for MprisPlayerService {
    type UpdateEvent = MprisPlayerEvent;
    type Error = ModuleError;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            MprisPlayerEvent::Refresh(data) => self.data = data,
            MprisPlayerEvent::Metadata(service, metadata) => {
                if let Some(entry) = self.data.iter_mut().find(|d| d.service == service) {
                    entry.metadata = metadata;
                }
            }
            MprisPlayerEvent::Volume(service, volume) => {
                if let Some(entry) = self.data.iter_mut().find(|d| d.service == service) {
                    entry.volume = volume;
                }
            }
            MprisPlayerEvent::State(service, state) => {
                if let Some(entry) = self.data.iter_mut().find(|d| d.service == service) {
                    entry.state = state;
                }
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::none()
    }
}

impl MprisPlayerService {
    /// Executes a command against the currently cached player list.
    pub(crate) async fn execute_command(
        service: Option<Self>,
        command: MprisPlayerCommand
    ) -> Result<Vec<MprisPlayerData>, ModuleError> {
        let service = service
            .ok_or_else(|| ModuleError::registration("MPRIS player service is not initialised"))?;

        execute_player_command(&service.conn, &service.data, command).await
    }
}

impl Service for MprisPlayerService {
    type Command = MprisPlayerCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let service = Some(self.clone());

        Task::perform(
            async move {
                match Self::execute_command(service, command).await {
                    Ok(data) => ServiceEvent::Update(MprisPlayerEvent::Refresh(data)),
                    Err(error) => ServiceEvent::Error(error)
                }
            },
            |event| event
        )
    }
}
