use tokio::{sync::mpsc::UnboundedSender, time::Duration};

use super::{backend::BackendCommand, model::AudioData};

/// Delay applied before attempting to reconnect to the backend after an error.
const RECONNECT_BACKOFF: Duration = Duration::from_millis(500);

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
    commander: UnboundedSender<BackendCommand>
}

mod commands;
mod listen;
mod traits;

#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests;
