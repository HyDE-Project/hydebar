//! Audio face of the control center: messages, the bar indicator, the
//! sliders and the device submenus.

use iced::SurfaceId as Id;

use crate::services::{ServiceEvent, audio::AudioService};

mod indicator;
mod sliders;
mod submenus;

pub use sliders::{SliderType, audio_slider};
pub use submenus::{SubmenuEntry, audio_submenu};

#[derive(Debug, Clone)]
pub enum AudioMessage {
    Event(Box<ServiceEvent<AudioService>>),
    DefaultSinkChanged(String, String),
    DefaultSourceChanged(String, String),
    ToggleSinkMute,
    SinkVolumeChanged(i32),
    /// A wheel notch over the bar entry, `1` up and `-1` down.
    SinkVolumeWheel(i32),
    ToggleSourceMute,
    SourceVolumeChanged(i32),
    SinksMore(Id),
    SourcesMore(Id)
}

/// The wheel notch as a volume direction: `1` up, `-1` down.
///
/// Stated once next to the message it feeds, so every place that takes the
/// wheel — the bar entry, the open menu — reads the same direction.
#[must_use]
pub fn wheel_direction(delta: iced::mouse::ScrollDelta) -> i32 {
    use iced::mouse::ScrollDelta;

    let up = match delta {
        ScrollDelta::Lines {
            y, ..
        }
        | ScrollDelta::Pixels {
            y, ..
        } => y > 0.0
    };

    if up { 1 } else { -1 }
}
