//! Commands, events and the handle used to drive an audio backend.

use std::{future::Future, pin::Pin, thread::JoinHandle};

use libpulse_binding::volume::ChannelVolumes;
use masterror::AppResult;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::PulseAudioServer;
use crate::services::audio::model::AudioEvent;

/// Commands accepted by backend implementations.
#[derive(Debug, Clone)]
pub enum BackendCommand {
    SinkMute(String, bool),
    SourceMute(String, bool),
    SinkVolume(String, ChannelVolumes),
    SourceVolume(String, ChannelVolumes),
    DefaultSink(String, String),
    DefaultSource(String, String)
}

/// Events emitted by backend implementations.
#[derive(Debug, Clone)]
pub enum BackendEvent {
    Error(String),
    Update(AudioEvent)
}

/// Future returned by backend spawners.
pub type BackendFuture = Pin<Box<dyn Future<Output = AppResult<BackendHandle>> + Send>>;

/// Abstraction over backend implementations to allow testing without
/// PulseAudio.
pub trait AudioBackend: Send + Sync + Clone + 'static {
    fn spawn(&self) -> BackendFuture;
}

/// Default PulseAudio backend implementation.
#[derive(Clone, Default)]
pub struct PulseAudioBackend;

impl AudioBackend for PulseAudioBackend {
    fn spawn(&self) -> BackendFuture {
        Box::pin(async { PulseAudioServer::start().await })
    }
}

/// Handle returned by [`AudioBackend::spawn`].
///
/// Keeps the listener and commander thread handles alive for the lifetime
/// of the backend. When dropped, the threads will be aborted.
#[derive(Debug)]
pub struct BackendHandle {
    pub(crate) receiver: UnboundedReceiver<BackendEvent>,
    pub(crate) sender:   UnboundedSender<BackendCommand>,
    _listener:           Option<JoinHandle<()>>,
    _commander:          Option<JoinHandle<()>>
}

impl BackendHandle {
    pub(super) fn new(
        receiver: UnboundedReceiver<BackendEvent>,
        sender: UnboundedSender<BackendCommand>,
        listener: JoinHandle<()>,
        commander: JoinHandle<()>
    ) -> Self {
        Self {
            receiver,
            sender,
            _listener: Some(listener),
            _commander: Some(commander)
        }
    }

    #[cfg(test)]
    pub(crate) fn from_parts(
        receiver: UnboundedReceiver<BackendEvent>,
        sender: UnboundedSender<BackendCommand>
    ) -> Self {
        Self {
            receiver,
            sender,
            _listener: None,
            _commander: None
        }
    }

    pub(crate) fn commander(&self) -> UnboundedSender<BackendCommand> {
        self.sender.clone()
    }

    pub(crate) async fn recv(&mut self) -> Option<BackendEvent> {
        self.receiver.recv().await
    }
}
