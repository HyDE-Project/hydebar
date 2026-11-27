//! `PulseAudio` backend for the audio service.

mod api;
mod control;
mod convert;
mod handle;
mod message;
mod server;
mod threads;

pub use api::{AudioBackend, PulseAudioBackend};
pub use handle::BackendHandle;
pub use message::{BackendCommand, BackendEvent};
