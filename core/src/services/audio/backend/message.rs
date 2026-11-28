//! Commands and events crossing the bridge to a running backend.

use libpulse_binding::volume::ChannelVolumes;

use crate::services::audio::model::AudioEvent;

/// Commands accepted by backend implementations.
#[derive(Debug, Clone)]
pub enum BackendCommand {
    /// Silence this output, or let it speak.
    SinkMute(String, bool),
    /// Silence this input, or let it hear.
    SourceMute(String, bool),
    /// Set the volume of this output.
    SinkVolume(String, ChannelVolumes),
    /// Set the volume of this input.
    SourceVolume(String, ChannelVolumes),
    /// Play everything through this output and port from now on.
    DefaultSink(String, String),
    /// Record everything from this input and port from now on.
    DefaultSource(String, String)
}

/// Events emitted by backend implementations.
#[derive(Debug, Clone)]
pub enum BackendEvent {
    /// The backend failed and says why.
    Error(String),
    /// Something about the sound server changed.
    Update(AudioEvent)
}
