//! `PulseAudio` backend for the audio service.

mod api;
mod control;
mod convert;
mod server;
mod threads;

pub use api::{AudioBackend, BackendCommand, BackendEvent, BackendHandle, PulseAudioBackend};
use server::PulseAudioServer;
