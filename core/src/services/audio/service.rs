use tokio::sync::mpsc::Sender;

use super::{backend::BackendCommand, model::AudioData};

/// Commands accepted by the audio service.
#[derive(Debug, Clone)]
pub enum AudioCommand {
    /// Silence the default output, or let it speak.
    ToggleSinkMute,
    /// Silence the default input, or let it hear.
    ToggleSourceMute,
    /// Set the volume of the default output.
    SinkVolume(i32),
    /// Set the volume of the default input.
    SourceVolume(i32),
    /// Play everything through this output from now on.
    DefaultSink(String, String),
    /// Record everything from this input from now on.
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
