//! Event stream assembled from `UPower` D-Bus signals.

use iced::futures::{
    Stream, StreamExt,
    stream::{once, select_all},
    stream_select
};
use log::error;
use masterror::{AppError, AppResult};
use zbus::zvariant::ObjectPath;

use super::{
    PowerProfile, State, UPowerEvent, UPowerService,
    dbus::{PowerProfilesProxy, UPowerDbus}
};
use crate::services::{ServiceEvent, ServiceEventPublisher};

impl UPowerService {
    async fn events(
        conn: &zbus::Connection,
        battery_devices: &Option<Vec<ObjectPath<'static>>>
    ) -> AppResult<impl Stream<Item = UPowerEvent> + use<>> {
        let battery_event = if let Some(battery_devices) = battery_devices {
            let upower = UPowerDbus::new(conn).await?;

            let mut events = Vec::new();

            for device_path in battery_devices {
                let device = upower.get_device(device_path).await?;

                events.push(
                    stream_select!(
                        device.receive_state_changed().await.map(|_| ()),
                        device.receive_percentage_changed().await.map(|_| ()),
                        device.receive_time_to_full_changed().await.map(|_| ()),
                        device.receive_time_to_empty_changed().await.map(|_| ()),
                    )
                    .filter_map({
                        let conn = conn.clone();
                        move |()| {
                            let conn = conn.clone();
                            async move {
                                if let Some((data, _)) =
                                    Self::initialize_battery_data(&conn).await.ok().flatten()
                                {
                                    Some(UPowerEvent::UpdateBattery(data))
                                } else {
                                    None
                                }
                            }
                        }
                    })
                    .boxed()
                );
            }

            select_all(events).boxed()
        } else {
            once(async {}).map(|()| UPowerEvent::NoBattery).boxed()
        };

        let powerprofiles = PowerProfilesProxy::new(conn).await.map_err(|e| {
            AppError::internal(format!(
                "Failed to create PowerProfilesProxy for events: {e}"
            ))
        })?;
        let power_profile_event =
            powerprofiles
                .receive_active_profile_changed()
                .await
                .map(move |_| {
                    UPowerEvent::UpdatePowerProfile(
                        powerprofiles
                            .cached_active_profile()
                            .map(|d| d.map(PowerProfile::from).unwrap_or_default())
                            .unwrap_or_default()
                    )
                });

        Ok(stream_select!(battery_event, power_profile_event))
    }

    pub(crate) async fn start_listening<P>(state: State, publisher: &mut P) -> State
    where
        P: ServiceEventPublisher<Self> + Send
    {
        match state {
            State::Init => match zbus::Connection::system()
                .await
                .map_err(|e| AppError::internal(format!("Failed to connect to system bus: {e}")))
            {
                Ok(conn) => {
                    let (battery, battery_path, power_profile) =
                        match Self::initialize_data(&conn).await {
                            Ok((Some((battery_data, battery_path)), power_profile)) => {
                                (Some(battery_data), Some(battery_path), power_profile)
                            }
                            Ok((None, power_profile)) => (None, None, power_profile),
                            Err(err) => {
                                error!("Failed to initialize upower service: {err}");

                                return State::Error;
                            }
                        };

                    let service = Self {
                        battery,
                        power_profile,
                        conn: conn.clone()
                    };
                    let () = publisher.send(ServiceEvent::Init(service)).await;

                    State::Active(conn, battery_path)
                }
                Err(err) => {
                    error!("Failed to connect to system bus for upower: {err}");
                    State::Error
                }
            },
            State::Active(conn, battery_devices) => {
                match Self::events(&conn, &battery_devices).await {
                    Ok(mut events) => {
                        while let Some(event) = events.next().await {
                            let () = publisher.send(ServiceEvent::Update(event)).await;
                        }

                        State::Active(conn, battery_devices)
                    }
                    Err(err) => {
                        error!("Failed to listen for upower events: {err}");

                        State::Error
                    }
                }
            }
            State::Error => {
                tokio::time::sleep(crate::services::RECONNECT_MAX_DELAY).await;

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
