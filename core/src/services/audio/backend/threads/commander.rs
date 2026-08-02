//! Commander thread applying volume and routing commands to `PulseAudio`.

use std::thread::{self, JoinHandle};

use iced::futures::executor::block_on;
use log::error;
use masterror::{AppError, AppResult};
use tokio::sync::mpsc::{Receiver, Sender};

use super::super::{BackendCommand, BackendEvent, PulseAudioServer};

impl PulseAudioServer {
    pub(in super::super) async fn start_commander(
        from_server_tx: Sender<BackendEvent>,
        mut to_server_rx: Receiver<BackendCommand>
    ) -> AppResult<JoinHandle<()>> {
        let (ready_tx, mut ready_rx) = tokio::sync::mpsc::channel(4);

        let handle = thread::spawn(move || {
            block_on(async move {
                match Self::new() {
                    Ok(mut server) => {
                        let _ = ready_tx.try_send(true);
                        while let Some(command) = to_server_rx.recv().await {
                            if let Err(err) = match command {
                                BackendCommand::SinkMute(name, mute) => {
                                    server.set_sink_mute(&name, mute)
                                }
                                BackendCommand::SourceMute(name, mute) => {
                                    server.set_source_mute(&name, mute)
                                }
                                BackendCommand::SinkVolume(name, volume) => {
                                    server.set_sink_volume(&name, &volume)
                                }
                                BackendCommand::SourceVolume(name, volume) => {
                                    server.set_source_volume(&name, &volume)
                                }
                                BackendCommand::DefaultSink(name, port) => {
                                    server.set_default_sink(&name, &port)
                                }
                                BackendCommand::DefaultSource(name, port) => {
                                    server.set_default_source(&name, &port)
                                }
                            } {
                                error!("PulseAudio command failed: {err}");
                            }
                        }
                    }
                    Err(err) => {
                        error!("Failed to start PulseAudio commander: {err}");
                        let _ = from_server_tx.try_send(BackendEvent::Error(err.to_string()));
                    }
                }
            });
        });

        match ready_rx.recv().await {
            Some(true) => Ok(handle),
            _ => Err(AppError::internal(
                "Failed to start PulseAudio commander thread"
            ))
        }
    }
}
