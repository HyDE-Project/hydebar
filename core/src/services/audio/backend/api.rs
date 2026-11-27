//! The backend abstraction the audio service spawns and drives.

use std::{future::Future, pin::Pin};

use masterror::AppResult;

use super::{handle::BackendHandle, server::PulseAudioServer};

/// Future returned by backend spawners.
pub type BackendFuture = Pin<Box<dyn Future<Output = AppResult<BackendHandle>> + Send>>;

/// Abstraction over backend implementations to allow testing without
/// `PulseAudio`.
pub trait AudioBackend: Send + Sync + Clone + 'static {
    /// Starts the backend and hands back the handle to its bridge.
    fn spawn(&self) -> BackendFuture;
}

/// Default `PulseAudio` backend implementation.
#[derive(Clone, Default)]
pub struct PulseAudioBackend;

impl AudioBackend for PulseAudioBackend {
    fn spawn(&self) -> BackendFuture {
        Box::pin(async { PulseAudioServer::start().await })
    }
}
