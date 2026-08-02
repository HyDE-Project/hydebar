//! Connecting to the system bus and picking a backend.

use iced::futures::TryFutureExt;
use log::{error, info};

use super::super::{
    BackendChoice, NetworkData, NetworkService, NetworkServiceError, State, gate::EventGate
};
use crate::services::{
    ServiceEvent, ServiceEventPublisher,
    network::backend::{NetworkBackend, iwd::IwdDbus, network_manager::NetworkDbus}
};

impl NetworkService {
    /// Connects to the bus, initializes a backend, and publishes the first
    /// snapshot.
    ///
    /// `NetworkManager` is tried first and iwd is the fallback; whichever
    /// answers becomes the backend for the active phase.
    pub(super) async fn initialize<P>(publisher: &mut P, gate: &mut EventGate) -> State
    where
        P: ServiceEventPublisher<Self> + Send
    {
        match zbus::Connection::system().await {
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
                        *gate = EventGate::new(&data);
                        let () = publisher
                            .send(ServiceEvent::Init(Self {
                                data,
                                conn: conn.clone(),
                                backend_choice: choice
                            }))
                            .await;
                        Self::publish_link_details(publisher).await;
                        State::Active(conn, choice)
                    }
                    Err(err) => {
                        if err.is::<zbus::Error>() {
                            error!("Failed to connect to system bus: {err}");
                        } else {
                            error!("Failed to initialize network service: {err}");
                        }
                        let error = NetworkServiceError::from(err);
                        let () = publisher.send(ServiceEvent::Error(error)).await;
                        State::Error
                    }
                }
            }
            Err(err) => {
                error!("Failed to connect to system bus: {err}");
                let error =
                    NetworkServiceError::new(format!("Failed to connect to system bus: {err}"));
                let () = publisher.send(ServiceEvent::Error(error)).await;

                State::Error
            }
        }
    }
}
