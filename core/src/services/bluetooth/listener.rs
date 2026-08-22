//! Listening loop reacting to bluez and `rfkill` change events.
//!
//! Two rooms: the loop itself is here, and [`events`] is the stream it waits
//! on — every source that can change what the adapter or one of its devices
//! reports, merged into a single nudge.

mod events;

use iced::futures::StreamExt;
use log::{error, info};
use masterror::AppResult;

use super::{BluetoothData, BluetoothService, BluetoothState, dbus::BluetoothDbus};
use crate::services::{ServiceEvent, ServiceEventPublisher};

pub(super) enum State {
    Init,
    Active(zbus::Connection),
    Error
}

impl BluetoothService {
    pub(super) async fn initialize_data(conn: &zbus::Connection) -> AppResult<BluetoothData> {
        let bluetooth = BluetoothDbus::new(conn).await?;

        let state = bluetooth.state().await?;
        let rfkill_soft_block = Self::check_rfkill_soft_block().await?;

        let state = match state {
            BluetoothState::Unavailable => BluetoothState::Unavailable,
            BluetoothState::Active if rfkill_soft_block => BluetoothState::Inactive,
            state => state
        };
        let devices = bluetooth.devices().await?;

        Ok(BluetoothData {
            state,
            devices
        })
    }

    async fn start_listening<P>(state: State, publisher: &mut P) -> State
    where
        P: ServiceEventPublisher<Self> + Send
    {
        match state {
            State::Init => match zbus::Connection::system().await {
                Ok(conn) => {
                    let data = Self::initialize_data(&conn).await;

                    match data {
                        Ok(data) => {
                            info!("Bluetooth service initialized");

                            let () = publisher
                                .send(ServiceEvent::Init(Self {
                                    data,
                                    conn: conn.clone()
                                }))
                                .await;

                            State::Active(conn)
                        }
                        Err(err) => {
                            error!("Failed to initialize bluetooth service: {err}");

                            State::Error
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to connect to system bus: {err}");

                    State::Error
                }
            },
            State::Active(conn) => {
                info!("Listening for bluetooth events");

                match Self::events(&conn).await {
                    Ok(mut events) => {
                        while events.next().await.is_some() {
                            if let Ok(data) = Self::initialize_data(&conn).await {
                                let () = publisher.send(ServiceEvent::Update(data)).await;
                            }
                        }

                        State::Active(conn)
                    }
                    Err(err) => {
                        error!("Failed to listen for bluetooth events: {err}");
                        State::Error
                    }
                }
            }
            State::Error => {
                error!("Bluetooth service error, retrying soon");

                State::Init
            }
        }
    }

    pub async fn listen<P>(publisher: &mut P)
    where
        P: ServiceEventPublisher<Self> + Send
    {
        let mut state = State::Init;
        let mut failures: u32 = 0;

        loop {
            state = Self::start_listening(state, publisher).await;

            match &state {
                State::Error => {
                    failures = failures.saturating_add(1);
                    tokio::time::sleep(crate::services::reconnect_delay(failures)).await;
                }
                State::Active(_) => failures = 0,
                State::Init => {}
            }
        }
    }
}
