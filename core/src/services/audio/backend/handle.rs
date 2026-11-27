//! Handle to a running backend: the bridge endpoints and the threads behind
//! them.

use std::thread::JoinHandle;

use tokio::sync::mpsc::{Receiver, Sender};

use super::message::{BackendCommand, BackendEvent};

/// Handle returned when a backend is spawned.
///
/// Keeps the listener and commander thread handles alive for the lifetime
/// of the backend. Dropping the handle closes both channels; the commander
/// exits on the closed command queue and the listener notices the closed
/// event queue on its next heartbeat and disconnects from the server.
#[derive(Debug)]
pub struct BackendHandle {
    receiver:   Receiver<BackendEvent>,
    sender:     Sender<BackendCommand>,
    _listener:  Option<JoinHandle<()>>,
    _commander: Option<JoinHandle<()>>
}

impl BackendHandle {
    /// Wraps the bridge endpoints and the threads owning them.
    pub(super) const fn new(
        receiver: Receiver<BackendEvent>,
        sender: Sender<BackendCommand>,
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

    /// The queue commands are sent through.
    pub(crate) fn commander(&self) -> Sender<BackendCommand> {
        self.sender.clone()
    }

    /// The next event the backend has to say, once there is one.
    pub(crate) async fn recv(&mut self) -> Option<BackendEvent> {
        self.receiver.recv().await
    }
}
