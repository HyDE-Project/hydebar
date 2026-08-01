pub mod error;
pub mod inotify;
pub mod pipewire;
pub mod publisher;

use std::{ops::Deref, pin::Pin};

pub use error::PrivacyError;
use iced::futures::Stream;
pub use publisher::PrivacyEventPublisher;
use tokio::sync::mpsc::Receiver;

const WEBCAM_DEVICE_PATH: &str = "/dev/video0";

pub(crate) type PrivacyStream = Pin<Box<dyn Stream<Item = PrivacyEvent> + Send>>;

mod data;
mod events;
mod listen;

#[cfg(test)]
mod tests;

pub use data::{ApplicationNode, Media, PrivacyData};

/// Service exposing read-only privacy state to interested modules.
#[derive(Debug, Clone)]
pub struct PrivacyService {
    data: PrivacyData
}

impl Deref for PrivacyService {
    type Target = PrivacyData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

pub(crate) enum State {
    Init,
    Active {
        pipewire: Receiver<PrivacyEvent>,
        webcam:   PrivacyStream
    }
}

/// Event emitted by the privacy service listeners.
#[derive(Debug, Clone)]
pub enum PrivacyEvent {
    /// A new `PipeWire` node has been announced.
    AddNode(ApplicationNode),
    /// A `PipeWire` node has been removed.
    RemoveNode(u32),
    /// The webcam device has been opened by an application.
    WebcamOpen,
    /// The webcam device has been closed by an application.
    WebcamClose
}
