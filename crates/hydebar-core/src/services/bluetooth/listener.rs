//! Listening loop reacting to bluez and `rfkill` change events.

use iced::futures::{Stream, StreamExt, stream::select_all, stream_select};
use log::{error, info};
use masterror::{AppError, AppResult};

use super::{
    BluetoothData, BluetoothService, BluetoothState,
    dbus::{BatteryProxy, BluetoothDbus}
};
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

    #[expect(
        clippy::needless_continue,
        reason = "the continue lives inside the stream_select macro expansion"
    )]
    async fn events(conn: &zbus::Connection) -> AppResult<impl Stream<Item = ()> + use<>> {
        let bluetooth = BluetoothDbus::new(conn).await?;

        let interface_changed = stream_select!(
            bluetooth
                .bluez
                .receive_interfaces_added()
                .await
                .map_err(|e| AppError::internal(format!(
                    "Failed to receive interfaces added: {e}"
                ),),)?
                .map(|_| {}),
            bluetooth
                .bluez
                .receive_interfaces_removed()
                .await
                .map_err(|e| AppError::internal(format!(
                    "Failed to receive interfaces removed: {e}"
                ),),)?
                .map(|_| {}),
        )
        .boxed();

        let combined = match bluetooth.adapter.as_ref() {
            Some(adapter) => {
                let powered = adapter.receive_powered_changed().await.map(|_| {});
                let rfkill = Self::listen_rfkill_soft_block_changes()?;
                let devices = bluetooth.devices().await?;

                let mut batteries = Vec::new();
                for device in devices.iter().filter(|d| d.battery.is_some()) {
                    let battery = BatteryProxy::builder(bluetooth.bluez.inner().connection())
                        .path(device.path.clone())
                        .map_err(|e| {
                            AppError::internal(format!("Failed to set battery path: {e}"))
                        })?
                        .build()
                        .await
                        .map_err(|e| {
                            AppError::internal(format!("Failed to build battery proxy: {e}"))
                        })?;
                    batteries.push(battery.receive_percentage_changed().await.map(|_| {}));
                }

                stream_select!(interface_changed, powered, rfkill, select_all(batteries)).boxed()
            }
            _ => interface_changed
        };

        Ok(combined)
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
