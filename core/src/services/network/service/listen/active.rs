//! Forwarding events from the chosen backend while it stays up.

use iced::futures::StreamExt;
use log::{debug, error, info};

use super::{
    super::{BackendChoice, NetworkService, NetworkServiceError, State, gate::EventGate},
    throttle::LinkThrottle
};
use crate::services::{
    ServiceEvent, ServiceEventPublisher,
    network::backend::{iwd::IwdDbus, network_manager::NetworkDbus}
};

impl NetworkService {
    /// Subscribes to the chosen backend's events and forwards them until the
    /// stream ends or fails.
    pub(super) async fn run_active<P>(
        conn: zbus::Connection,
        choice: BackendChoice,
        publisher: &mut P,
        gate: &mut EventGate
    ) -> State
    where
        P: ServiceEventPublisher<Self> + Send
    {
        info!("Listening for network events");

        match choice {
            BackendChoice::NetworkManager => {
                let nm = match NetworkDbus::new(&conn).await {
                    Ok(nm) => nm,
                    Err(e) => {
                        error!("Failed to create NetworkDbus: {e}");
                        let error = NetworkServiceError::from(e);
                        let () = publisher.send(ServiceEvent::Error(error)).await;
                        return State::Error;
                    }
                };

                match nm.subscribe_events().await {
                    Ok(events) => {
                        match Self::consume_network_events(events, publisher, gate).await {
                            Ok(()) => {
                                debug!("Network service exit events stream");
                                State::Active(conn, choice)
                            }
                            Err(err) => {
                                error!("Network event stream error: {err}");
                                let error = NetworkServiceError::from(err);
                                let () = publisher.send(ServiceEvent::Error(error)).await;
                                State::Error
                            }
                        }
                    }
                    Err(err) => {
                        error!("Failed to listen for network events: {err}");
                        let error = NetworkServiceError::from(err);
                        let () = publisher.send(ServiceEvent::Error(error)).await;

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
                        let () = publisher.send(ServiceEvent::Error(error)).await;
                        return State::Error;
                    }
                };
                match iwd.subscribe_events().await {
                    Ok(mut event_s) => {
                        let mut throttle = LinkThrottle::default();

                        while let Some(events) = event_s.next().await {
                            for event in events {
                                if gate.admits(&event) {
                                    let refresh_link = Self::moves_the_link(&event, &mut throttle);
                                    let () = publisher.send(ServiceEvent::Update(event)).await;

                                    if refresh_link {
                                        Self::publish_link_details(publisher).await;
                                    }
                                }
                            }
                        }

                        debug!("Network service exit events stream");

                        State::Active(conn, choice)
                    }
                    Err(err) => {
                        error!("Failed to listen for network events: {err}");
                        let error = NetworkServiceError::from(err);
                        let () = publisher.send(ServiceEvent::Error(error)).await;

                        State::Error
                    }
                }
            }
        }
    }
}
