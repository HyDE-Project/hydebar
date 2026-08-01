use std::{future::Future, ops::Deref, pin::Pin};

use commands::{execute_player_command, module_error};
use futures::StreamExt;
use iced::{Subscription, Task};
use log::{debug, error, info};
use zbus::Connection;

use super::{ReadOnlyService, Service, ServiceEvent};
use crate::modules::ModuleError;

mod commands;
pub mod data;
mod dbus;
mod ipc;

pub use commands::{MprisPlayerCommand, PlayerCommand};
pub use data::{MprisPlayerData, MprisPlayerEvent, MprisPlayerMetadata, PlaybackStatus};
use ipc::{IpcEvent, build_event_stream, collect_players};

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

/// Publishes events emitted by the MPRIS service.
pub(crate) trait MprisEventPublisher {
    /// Sends a [`ServiceEvent`] to consumers.
    fn send(
        &mut self,
        event: ServiceEvent<MprisPlayerService>
    ) -> Pin<Box<dyn Future<Output = Result<(), ModuleError>> + Send + '_>>;
}

/// Internal state machine for the MPRIS listener runtime.
#[derive(Debug, Clone)]
pub(crate) enum ListenerState {
    /// No connection has been established yet.
    Init,
    /// The service is actively listening for events on the provided connection.
    Active(Connection)
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
    /// Starts or resumes the MPRIS listener depending on the provided `state`.
    pub(crate) async fn start_listening<P>(
        state: ListenerState,
        publisher: &mut P
    ) -> Result<ListenerState, ModuleError>
    where
        P: MprisEventPublisher
    {
        Self::start_listening_internal(state, publisher).await
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one state machine with an arm per listener phase; splitting the arms would separate them from the retry flow"
    )]
    async fn start_listening_internal<P>(
        state: ListenerState,
        publisher: &mut P
    ) -> Result<ListenerState, ModuleError>
    where
        P: MprisEventPublisher
    {
        match state {
            ListenerState::Init => {
                let conn = Connection::session()
                    .await
                    .map_err(|err| module_error("failed to connect to session bus", err))?;

                match collect_players(&conn).await {
                    Ok(data) => {
                        info!("MPRIS player service initialized");

                        publisher
                            .send(ServiceEvent::Init(Self {
                                data,
                                conn: conn.clone()
                            }))
                            .await?;

                        Ok(ListenerState::Active(conn))
                    }
                    Err(err) => {
                        error!("Failed to initialize MPRIS player service: {err}");
                        Err(module_error(
                            "failed to initialize MPRIS player service",
                            err
                        ))
                    }
                }
            }
            ListenerState::Active(conn) => match build_event_stream(&conn).await {
                Ok(events) => {
                    let mut chunks = events.ready_chunks(10);

                    while let Some(chunk) = chunks.next().await {
                        debug!("MPRIS player service receive events: {chunk:?}");

                        let mut need_refresh = false;

                        for event in chunk {
                            match event {
                                IpcEvent::NameOwner => {
                                    debug!("MPRIS player service name owner changed");
                                    need_refresh = true;
                                }
                                IpcEvent::Metadata(service, metadata) => {
                                    debug!(
                                        "MPRIS player service {service} metadata changed: {metadata:?}"
                                    );
                                    publisher
                                        .send(ServiceEvent::Update(MprisPlayerEvent::Metadata(
                                            service, metadata
                                        )))
                                        .await?;
                                }
                                IpcEvent::Volume(service, volume) => {
                                    debug!(
                                        "MPRIS player service {service} volume changed: {volume:?}"
                                    );
                                    publisher
                                        .send(ServiceEvent::Update(MprisPlayerEvent::Volume(
                                            service, volume
                                        )))
                                        .await?;
                                }
                                IpcEvent::State(service, state) => {
                                    debug!(
                                        "MPRIS player service {service} playback status changed: {state:?}"
                                    );
                                    publisher
                                        .send(ServiceEvent::Update(MprisPlayerEvent::State(
                                            service, state
                                        )))
                                        .await?;
                                }
                            }
                        }

                        if need_refresh {
                            match collect_players(&conn).await {
                                Ok(data) => {
                                    debug!("Refreshing MPRIS player data");
                                    publisher
                                        .send(ServiceEvent::Update(MprisPlayerEvent::Refresh(
                                            data
                                        )))
                                        .await?;
                                }
                                Err(err) => {
                                    error!("Failed to fetch MPRIS player data: {err}");
                                    return Err(module_error(
                                        "failed to refresh MPRIS player data",
                                        err
                                    ));
                                }
                            }

                            break;
                        }
                    }

                    Ok(ListenerState::Active(conn))
                }
                Err(err) => {
                    error!("Failed to listen for MPRIS player events: {err}");
                    Err(module_error(
                        "failed to listen for MPRIS player events",
                        err
                    ))
                }
            }
        }
    }

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

// TODO: Fix broken tests
