//! Connection lifecycle and event forwarding.

use std::time::Duration;

use iced::futures::{Stream, StreamExt, TryFutureExt};
use log::{debug, error, info};
use masterror::AppResult;
use tokio::time::sleep;

use super::{
    super::{
        NetworkData, NetworkEvent, NetworkService, NetworkServiceError,
        backend::{NetworkBackend, iwd::IwdDbus, network_manager::NetworkDbus}
    },
    BackendChoice, State
};
use crate::services::{ServiceEvent, ServiceEventPublisher};

impl NetworkService {
    /// Records a recoverable error on the network service state.
    ///
    /// # Examples
    /// ```
    /// use std::ops::Deref;
    ///
    /// use hydebar_core::services::network::{NetworkService, NetworkServiceError};
    ///
    /// fn inspect(service: &NetworkService) -> Option<&NetworkServiceError> {
    ///     service.deref().last_error.as_ref()
    /// }
    ///
    /// # fn exercise(mut service: NetworkService) {
    /// service.apply_error(NetworkServiceError::new("unreachable"));
    /// assert!(inspect(&service).is_some());
    /// # }
    /// ```
    pub fn apply_error(&mut self, error: NetworkServiceError) {
        self.data.last_error = Some(error);
    }

    pub(super) async fn consume_network_events<S, P>(
        mut events: S,
        publisher: &mut P
    ) -> AppResult<()>
    where
        S: Stream<Item = AppResult<NetworkEvent>> + Unpin,
        P: ServiceEventPublisher<Self> + Send
    {
        while let Some(event) = events.next().await {
            let event = event?;
            let mut exit_loop = false;
            if let NetworkEvent::WirelessDevice {
                ..
            } = event
            {
                exit_loop = true;
            }
            let _ = publisher.send(ServiceEvent::Update(event)).await;

            if exit_loop {
                break;
            }
        }

        Ok(())
    }

    pub(super) async fn start_listening<P>(state: State, publisher: &mut P) -> State
    where
        P: ServiceEventPublisher<Self> + Send
    {
        match state {
            State::Init => match zbus::Connection::system().await {
                Ok(conn) => {
                    info!("Connecting to backend");
                    let maybe_backend: Result<(NetworkData, BackendChoice), _> =
                        match NetworkDbus::new(&conn)
                            .and_then(|nm| async move { nm.initialize_data().await })
                            .await
                        {
                            Ok(data) => {
                                info!("NetworkManager service initialized");
                                Ok((data, BackendChoice::NetworkManager))
                            }
                            Err(err) => {
                                info!(
                                    "Failed to initialize NetworkManager. Falling back to iwd. Error: {err}"
                                );
                                match IwdDbus::new(&conn)
                                    .and_then(|iwd| async move { iwd.initialize_data().await })
                                    .await
                                {
                                    Ok(data) => {
                                        info!("IWD service initialized");
                                        Ok((data, BackendChoice::Iwd))
                                    }
                                    Err(err) => {
                                        error!("Failed to initialize network service: {err}");
                                        Err(err)
                                    }
                                }
                            }
                        };
                    info!("Connected");

                    match maybe_backend {
                        Ok((data, choice)) => {
                            info!("Network service initialized");
                            let _ = publisher
                                .send(ServiceEvent::Init(NetworkService {
                                    data,
                                    conn: conn.clone(),
                                    backend_choice: choice
                                }))
                                .await;
                            State::Active(conn, choice)
                        }
                        Err(err) => {
                            if err.is::<zbus::Error>() {
                                error!("Failed to connect to system bus: {err}");
                            } else {
                                error!("Failed to initialize network service: {err}");
                            }
                            let error = NetworkServiceError::from(err);
                            let _ = publisher.send(ServiceEvent::Error(error)).await;
                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to connect to system bus: {err}");
                    let error = NetworkServiceError::new(format!(
                        "Failed to connect to system bus: {err}"
                    ));
                    let _ = publisher.send(ServiceEvent::Error(error)).await;

                    State::Error
                }
            },
            State::Active(conn, choice) => {
                info!("Listening for network events");

                match choice {
                    BackendChoice::NetworkManager => {
                        let nm = match NetworkDbus::new(&conn).await {
                            Ok(nm) => nm,
                            Err(e) => {
                                error!("Failed to create NetworkDbus: {e}");
                                let error = NetworkServiceError::from(e);
                                let _ = publisher.send(ServiceEvent::Error(error)).await;
                                return State::Error;
                            }
                        };

                        match nm.subscribe_events().await {
                            Ok(events) => {
                                match Self::consume_network_events(events, publisher).await {
                                    Ok(()) => {
                                        debug!("Network service exit events stream");
                                        State::Active(conn, choice)
                                    }
                                    Err(err) => {
                                        error!("Network event stream error: {err}");
                                        let error = NetworkServiceError::from(err);
                                        let _ = publisher.send(ServiceEvent::Error(error)).await;
                                        State::Error
                                    }
                                }
                            }
                            Err(err) => {
                                error!("Failed to listen for network events: {err}");
                                let error = NetworkServiceError::from(err);
                                let _ = publisher.send(ServiceEvent::Error(error)).await;

                                State::Error
                            }
                        }
                    }
                    BackendChoice::Iwd => {
                        let iwd = match IwdDbus::new(&conn).await {
                            Ok(iwd) => iwd,
                            Err(err) => {
                                error!("Failed to create IwdDbus: {err}");
                                let error = NetworkServiceError::from(err);
                                let _ = publisher.send(ServiceEvent::Error(error)).await;
                                return State::Error;
                            }
                        };
                        match iwd.subscribe_events().await {
                            Ok(mut event_s) => {
                                while let Some(events) = event_s.next().await {
                                    for event in events {
                                        let _ = publisher.send(ServiceEvent::Update(event)).await;
                                    }
                                }

                                debug!("Network service exit events stream");

                                State::Active(conn, choice)
                            }
                            Err(err) => {
                                error!("Failed to listen for network events: {err}");
                                let error = NetworkServiceError::from(err);
                                let _ = publisher.send(ServiceEvent::Error(error)).await;

                                State::Error
                            }
                        }
                    }
                }
            }
            State::Error => {
                error!("Network service error");

                sleep(Duration::from_secs(1)).await;

                State::Init
            }
        }
    }

    pub async fn listen<P>(publisher: &mut P)
    where
        P: ServiceEventPublisher<Self> + Send
    {
        let mut state = State::Init;

        loop {
            state = Self::start_listening(state, publisher).await;
        }
    }
}
