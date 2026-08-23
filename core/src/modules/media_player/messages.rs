//! Messages accepted by the media player module.

use crate::services::{ServiceEvent, mpris::MprisPlayerService};

/// What the media entry answers to.
#[derive(Debug, Clone)]
pub enum Message {
    /// Go back a track, in this player.
    Prev(String),
    /// Play or pause, in this player.
    PlayPause(String),
    /// Go on a track, in this player.
    Next(String),
    /// Set the volume of this player.
    SetVolume(String, f64),
    /// The media bus said something.
    Event(Box<ServiceEvent<MprisPlayerService>>)
}
