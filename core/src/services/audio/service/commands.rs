//! Translation of user commands into backend messages.

use log::error;

use super::super::{AudioCommand, AudioService, backend::BackendCommand, model::Volume};
use crate::services::ServiceEvent;

impl AudioService {
    fn send_backend_command(&self, command: BackendCommand) {
        if let Err(err) = self.commander.try_send(command) {
            error!("Failed to dispatch audio command: {err}");
        }
    }

    pub(super) fn apply_command(&mut self, command: AudioCommand) {
        match command {
            AudioCommand::ToggleSinkMute => {
                if let Some(sink) = self
                    .data
                    .sinks
                    .iter()
                    .find(|sink| sink.name == self.data.server_info.default_sink)
                {
                    self.send_backend_command(BackendCommand::SinkMute(
                        sink.name.clone(),
                        !sink.is_mute
                    ));
                }
            }
            AudioCommand::ToggleSourceMute => {
                if let Some(source) = self
                    .data
                    .sources
                    .iter()
                    .find(|source| source.name == self.data.server_info.default_source)
                {
                    self.send_backend_command(BackendCommand::SourceMute(
                        source.name.clone(),
                        !source.is_mute
                    ));
                }
            }
            AudioCommand::SinkVolume(volume) => {
                let command = self
                    .data
                    .sinks
                    .iter_mut()
                    .find(|sink| sink.name == self.data.server_info.default_sink)
                    .and_then(|sink| {
                        sink.volume
                            .scale_volume(f64::from(volume) / 100.0)
                            .map(|volume| BackendCommand::SinkVolume(sink.name.clone(), *volume))
                    });

                if let Some(command) = command {
                    self.send_backend_command(command);
                }
            }
            AudioCommand::SourceVolume(volume) => {
                let command = self
                    .data
                    .sources
                    .iter_mut()
                    .find(|source| source.name == self.data.server_info.default_source)
                    .and_then(|source| {
                        source
                            .volume
                            .scale_volume(f64::from(volume) / 100.0)
                            .map(|volume| {
                                BackendCommand::SourceVolume(source.name.clone(), *volume)
                            })
                    });

                if let Some(command) = command {
                    self.send_backend_command(command);
                }
            }
            AudioCommand::DefaultSink(name, port) => {
                self.send_backend_command(BackendCommand::DefaultSink(name, port));
            }
            AudioCommand::DefaultSource(name, port) => {
                self.send_backend_command(BackendCommand::DefaultSource(name, port));
            }
        }
    }

    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "kept async to match the service command runner signature expected by the control center"
    )]
    /// Carries out one command and says what came of it.
    pub async fn run_command(mut self, command: AudioCommand) -> Option<ServiceEvent<Self>> {
        self.apply_command(command);
        None
    }
}
