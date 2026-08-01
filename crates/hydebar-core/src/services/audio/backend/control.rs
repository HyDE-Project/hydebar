//! Device control operations issued to the `PulseAudio` server.

use libpulse_binding::{
    callbacks::ListResult,
    context::introspect::{SinkInfo, SourceInfo},
    def::PortAvailable,
    mainloop::standard::IterateResult,
    operation::{self, Operation},
    volume::ChannelVolumes
};
use log::{debug, error, trace};
use masterror::{AppError, AppResult};
use tokio::sync::mpsc::Sender;

use super::{BackendEvent, PulseAudioServer};
use crate::services::audio::model::{AudioEvent, Device};

impl PulseAudioServer {
    pub(super) fn wait_for_response<T: ?Sized>(
        &mut self,
        operation: Operation<T>
    ) -> AppResult<()> {
        loop {
            match self.mainloop.iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    error!("PulseAudio iterate failure");
                    return Err(AppError::internal("PulseAudio iterate failure"));
                }
                IterateResult::Success(_) => {
                    if operation.get_state() == operation::State::Done {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    pub(super) fn send_server_info(
        info: &libpulse_binding::context::introspect::ServerInfo<'_>,
        tx: &Sender<BackendEvent>
    ) {
        let _ = tx.try_send(BackendEvent::Update(AudioEvent::ServerInfo(info.into())));
    }

    pub(super) fn populate_and_send_sinks(
        info: ListResult<&SinkInfo<'_>>,
        tx: &Sender<BackendEvent>,
        sinks: &mut Vec<Device>
    ) {
        match info {
            ListResult::Item(data) => {
                if data
                    .ports
                    .iter()
                    .any(|port| port.available != PortAvailable::No)
                {
                    debug!("Adding sink data: {data:?}");
                    sinks.push(data.into());
                }
            }
            ListResult::End => {
                debug!("New sink list {sinks:?}");
                let _ = tx.try_send(BackendEvent::Update(AudioEvent::Sinks(sinks.clone())));
                sinks.clear();
            }
            ListResult::Error => error!("Error during sink list population")
        }
    }

    pub(super) fn populate_and_send_sources(
        info: ListResult<&SourceInfo<'_>>,
        tx: &Sender<BackendEvent>,
        sources: &mut Vec<Device>
    ) {
        match info {
            ListResult::Item(data) => {
                trace!("Received source data: {data:?}");

                if data
                    .name
                    .as_ref()
                    .is_some_and(|name| !name.contains("monitor"))
                {
                    debug!("Adding source data: {data:?}");
                    sources.push(data.into());
                }
            }
            ListResult::End => {
                debug!("New sources list {sources:?}");
                let _ = tx.try_send(BackendEvent::Update(AudioEvent::Sources(sources.clone())));
                sources.clear();
            }
            ListResult::Error => error!("Error during sources list population")
        }
    }

    pub(super) fn set_sink_mute(&mut self, name: &str, mute: bool) -> AppResult<()> {
        let op = self.introspector.set_sink_mute_by_name(name, mute, None);
        self.wait_for_response(op)
    }

    pub(super) fn set_source_mute(&mut self, name: &str, mute: bool) -> AppResult<()> {
        let op = self.introspector.set_source_mute_by_name(name, mute, None);
        self.wait_for_response(op)
    }

    pub(super) fn set_sink_volume(
        &mut self,
        name: &str,
        volume: &ChannelVolumes
    ) -> AppResult<()> {
        let op = self
            .introspector
            .set_sink_volume_by_name(name, volume, None);
        self.wait_for_response(op)
    }

    pub(super) fn set_source_volume(
        &mut self,
        name: &str,
        volume: &ChannelVolumes
    ) -> AppResult<()> {
        let op = self
            .introspector
            .set_source_volume_by_name(name, volume, None);
        self.wait_for_response(op)
    }

    pub(super) fn set_default_sink(&mut self, name: &str, port: &str) -> AppResult<()> {
        let op = self.context.set_default_sink(name, |_| {});
        self.wait_for_response(op)?;

        let op = self.introspector.set_sink_port_by_name(name, port, None);
        self.wait_for_response(op)
    }

    pub(super) fn set_default_source(&mut self, name: &str, port: &str) -> AppResult<()> {
        let op = self.context.set_default_source(name, |_| {});
        self.wait_for_response(op)?;

        let op = self.introspector.set_source_port_by_name(name, port, None);
        self.wait_for_response(op)
    }
}
