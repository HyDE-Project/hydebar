//! Listening loop reacting to udev backlight change events.

use std::path::PathBuf;

use log::{debug, error, info, warn};

use super::{BrightnessError, BrightnessEvent, BrightnessService};
use crate::services::{ServiceEvent, ServiceEventPublisher};

pub(super) enum State {
    Init,
    Active(PathBuf),
    Error
}

impl BrightnessService {
    async fn init_service() -> Result<(zbus::Connection, PathBuf), BrightnessError> {
        let backlight_devices = Self::backlight_enumerate()?;
        let candidate = backlight_devices
            .iter()
            .find(|device| device.subsystem().and_then(|s| s.to_str()) == Some("backlight"));
        let device_path =
            match Self::resolve_device_path(candidate.map(|d| d.syspath().to_path_buf())) {
                Ok(path) => path,
                Err(err @ BrightnessError::MissingDevice) => {
                    warn!("No backlight devices found");
                    return Err(err);
                }
                Err(err) => return Err(err)
            };

        let conn = zbus::Connection::system()
            .await
            .map_err(BrightnessError::from)?;

        Ok((conn, device_path))
    }

    async fn start_listening<P>(state: State, publisher: &mut P) -> Result<State, BrightnessError>
    where
        P: ServiceEventPublisher<Self> + Send
    {
        match state {
            State::Init => {
                let (conn, device_path) = Self::init_service().await?;
                let data = Self::initialize_data(&device_path)?;
                let service = Self {
                    data,
                    device_path: device_path.clone(),
                    conn
                };
                let () = publisher.send(ServiceEvent::Init(service)).await;

                Ok(State::Active(device_path))
            }
            State::Active(device_path) => {
                info!("Listening for brightness events");
                let mut current_value = Self::get_actual_brightness(&device_path)?;
                let mut socket = Self::backlight_monitor_listener()?;

                loop {
                    let mut guard = socket.writable_mut().await.map_err(BrightnessError::from)?;

                    for evt in guard.get_inner().iter() {
                        debug!("{:?}: {:?}", evt.event_type(), evt.device());

                        if evt.device().subsystem().and_then(|s| s.to_str()) != Some("backlight") {
                            continue;
                        }

                        match evt.event_type() {
                            udev::EventType::Change => {
                                debug!("Changed backlight device: {}", evt.syspath().display());
                                let new_value = Self::get_actual_brightness(&device_path)?;

                                if new_value != current_value {
                                    current_value = new_value;
                                    let () = publisher
                                        .send(ServiceEvent::Update(BrightnessEvent(new_value)))
                                        .await;
                                }
                            }
                            other => {
                                debug!("Unhandled event type: {other:?}");
                            }
                        }
                    }

                    guard.clear_ready();
                }

                #[allow(unreachable_code)]
                Ok(State::Active(device_path))
            }
            State::Error => {
                error!("Brightness service error, retrying soon");
                Ok(State::Init)
            }
        }
    }

    /// Keeps the conversation with the backlight for the whole session.
    pub async fn listen<P>(publisher: &mut P)
    where
        P: ServiceEventPublisher<Self> + Send
    {
        let mut state = State::Init;
        let mut failures: u32 = 0;

        loop {
            match Self::start_listening(state, publisher).await {
                Ok(next_state) => {
                    state = next_state;
                }
                Err(BrightnessError::MissingDevice) => {
                    info!("Brightness service: no backlight devices available, service disabled");
                    return;
                }
                Err(err) => {
                    error!("Brightness service failure: {err:?}");
                    let () = publisher.send(ServiceEvent::Error(err.clone())).await;
                    state = State::Error;
                }
            }

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
