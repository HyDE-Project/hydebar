//! Messages accepted by the media player module.

use crate::services::{ServiceEvent, mpris::MprisPlayerService};

#[derive(Debug, Clone)]
pub enum Message {
    Prev(String),
    PlayPause(String),
    Next(String),
    SetVolume(String, f64),
    Event(Box<ServiceEvent<MprisPlayerService>>)
}
