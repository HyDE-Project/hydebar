//! Listener state machine reacting to MPRIS bus events.

use std::{future::Future, pin::Pin};

use futures::StreamExt;
use log::{debug, error, info};
use zbus::Connection;

use super::{
    MprisPlayerEvent, MprisPlayerService,
    commands::module_error,
    ipc::{IpcEvent, build_event_stream, collect_players}
};
use crate::{modules::ModuleError, services::ServiceEvent};

/// Publishes events emitted by the MPRIS service.
pub trait MprisEventPublisher {
    /// Sends a [`ServiceEvent`] to consumers.
    fn send(
        &mut self,
        event: ServiceEvent<MprisPlayerService>
    ) -> Pin<Box<dyn Future<Output = Result<(), ModuleError>> + Send + '_>>;
}

/// Internal state machine for the MPRIS listener runtime.
#[derive(Debug, Clone)]
pub enum ListenerState {
    /// No connection has been established yet.
    Init,
    /// The service is actively listening for events on the provided connection.
    Active(Connection)
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
}
