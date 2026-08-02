use tokio::sync::mpsc::Sender;

use super::{backend::BackendCommand, model::AudioData};

/// Commands accepted by the audio service.
#[derive(Debug, Clone)]
pub enum AudioCommand {
    ToggleSinkMute,
    ToggleSourceMute,
    SinkVolume(i32),
    SourceVolume(i32),
    DefaultSink(String, String),
    DefaultSource(String, String)
}

/// Read/write handle to the audio state and command channel.
#[derive(Debug, Clone)]
pub struct AudioService {
    data:      AudioData,
    commander: Sender<BackendCommand>
}

mod commands;
mod listen;
mod traits;
