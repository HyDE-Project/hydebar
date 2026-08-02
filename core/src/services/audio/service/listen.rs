//! Backend connection lifecycle and event application.

use log::{error, warn};
use tokio::time::sleep;

use super::super::{
    AudioService,
    backend::{AudioBackend, BackendEvent, BackendHandle, PulseAudioBackend},
    model::{AudioData, AudioEvent, Device, Volume}
};
use crate::services::{ServiceEvent, ServiceEventPublisher};

pub(super) enum State {
    Init,
    Active(BackendHandle),
    Error
}

impl AudioService {
    async fn listen_with_backend<P, B>(backend: B, publisher: &mut P)
    where
        P: ServiceEventPublisher<Self> + Send,
        B: AudioBackend
    {
        let mut state = State::Init;
        let backend = backend;

        let mut failures: u32 = 0;

        loop {
            state = Self::start_listening(&backend, state, publisher).await;

            match &state {
                State::Error => {
                    failures = failures.saturating_add(1);
                    sleep(crate::services::reconnect_delay(failures)).await;
                }
                State::Active(_) => failures = 0,
                State::Init => {}
            }
        }
    }

    async fn start_listening<P, B>(backend: &B, state: State, publisher: &mut P) -> State
    where
        P: ServiceEventPublisher<Self> + Send,
        B: AudioBackend
    {
        match state {
            State::Init => match backend.spawn().await {
                Ok(handle) => {
                    let () = publisher
                        .send(ServiceEvent::Init(Self {
                            data:      AudioData::default(),
                            commander: handle.commander()
                        }))
                        .await;

                    State::Active(handle)
                }
                Err(err) => {
                    error!("Failed to initialise audio backend: {err}");
                    let () = publisher.send(ServiceEvent::Error(())).await;
                    State::Error
                }
            },
            State::Active(mut handle) => match handle.recv().await {
                Some(BackendEvent::Error(err)) => {
                    error!("Audio backend error: {err}");
                    let () = publisher.send(ServiceEvent::Error(())).await;
                    State::Error
                }
                Some(BackendEvent::Update(event)) => {
                    let () = publisher.send(ServiceEvent::Update(event)).await;
                    State::Active(handle)
                }
                None => {
                    warn!("Audio backend closed event stream");
                    let () = publisher.send(ServiceEvent::Error(())).await;
                    State::Error
                }
            },
            State::Error => State::Init
        }
    }

    pub(super) fn update_from_event(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::Sinks(sinks) => {
                self.data.sinks = sinks;
                self.data.cur_sink_volume = Self::active_device_volume(
                    &self.data.sinks,
                    &self.data.server_info.default_sink
                );
            }
            AudioEvent::Sources(sources) => {
                self.data.sources = sources;
                self.data.cur_source_volume = Self::active_device_volume(
                    &self.data.sources,
                    &self.data.server_info.default_source
                );
            }
            AudioEvent::ServerInfo(info) => {
                self.data.server_info = info;
                self.data.cur_sink_volume = Self::active_device_volume(
                    &self.data.sinks,
                    &self.data.server_info.default_sink
                );
                self.data.cur_source_volume = Self::active_device_volume(
                    &self.data.sources,
                    &self.data.server_info.default_source
                );
            }
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "the normalized volume scaled to percent is far below i32::MAX; fractional percent is intentionally dropped"
    )]
    pub(super) fn active_device_volume(devices: &[Device], default: &str) -> i32 {
        let volume = devices
            .iter()
            .find_map(|device| {
                if device
                    .ports
                    .iter()
                    .any(|port| port.active && device.name == default)
                {
                    Some(if device.is_mute {
                        0.0
                    } else {
                        device.volume.get_volume()
                    })
                } else {
                    None
                }
            })
            .unwrap_or_default();

        (volume * 100.0) as i32
    }

    pub async fn listen<P>(publisher: &mut P)
    where
        P: ServiceEventPublisher<Self> + Send
    {
        Self::listen_with_backend(PulseAudioBackend, publisher).await;
    }
}
