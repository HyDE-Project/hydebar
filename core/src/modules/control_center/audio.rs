//! Audio face of the control center: messages, the bar indicator, the
//! sliders and the device submenus.

use iced::SurfaceId as Id;

use crate::services::{ServiceEvent, audio::AudioService};

mod indicator;
mod sliders;
mod submenus;

pub use sliders::{SliderType, audio_slider};
pub use submenus::{SubmenuEntry, audio_submenu};

/// What the sound section of the quick settings answers to.
#[derive(Debug, Clone)]
pub enum AudioMessage {
    /// The sound server said something.
    Event(Box<ServiceEvent<AudioService>>),
    /// Play everything through this output from now on.
    DefaultSinkChanged(String, String),
    /// Record everything from this input from now on.
    DefaultSourceChanged(String, String),
    /// Silence the default output, or let it speak.
    ToggleSinkMute,
    /// Set the volume of the default output.
    SinkVolumeChanged(i32),
    /// A wheel notch over the bar entry, `1` up and `-1` down.
    SinkVolumeWheel(i32),
    /// Silence the default input, or let it hear.
    ToggleSourceMute,
    /// Set the volume of the default input.
    SourceVolumeChanged(i32),
    /// Open the full output settings.
    SinksMore(Id),
    /// Open the full input settings.
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
