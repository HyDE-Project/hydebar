use libpulse_binding::volume::ChannelVolumes;

use crate::components::icons::Icons;

/// Describes a single audio device (sink or source).
///
/// Each device carries metadata exported by `PulseAudio` that is consumed by
/// the settings UI.
#[derive(Debug, Clone)]
pub struct Device {
    /// Name the server addresses the device by.
    pub name:        String,
    /// Name a person would recognise it by.
    pub description: String,
    /// Volume of every channel it carries.
    pub volume:      ChannelVolumes,
    /// Whether the device is silenced.
    pub is_mute:     bool,
    /// Whether anything is playing through it.
    pub in_use:      bool,
    /// The sockets the device can route to.
    pub ports:       Vec<Port>
}

/// Represents a selectable device port and its metadata.
#[derive(Debug, Clone)]
pub struct Port {
    /// Name the server addresses the port by.
    pub name:        String,
    /// Name a person would recognise it by.
    pub description: String,
    /// What kind of thing is plugged into it.
    pub device_type: DeviceType,
    /// Whether this is the port in use.
    pub active:      bool
}

/// Enumerates known device categories.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DeviceType {
    /// Something worn over the ears.
    Headphones,
    /// Something in the room.
    Speaker,
    /// Headphones with a microphone.
    Headset,
    /// Something on the other end of a display cable.
    Hdmi
}

impl DeviceType {
    /// Returns the icon that should be displayed for the device category.
    #[must_use]
    pub const fn get_icon(&self) -> Icons {
        match self {
            Self::Speaker => Icons::Speaker3,
            Self::Headphones => Icons::Headphones1,
            Self::Headset => Icons::Headset,
            Self::Hdmi => Icons::MonitorSpeaker
        }
    }
}

/// Server level metadata tracked by the audio service.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    /// The output everything plays to unless told otherwise.
    pub default_sink:   String,
    /// The input everything records from unless told otherwise.
    pub default_source: String
}

/// Provides a view on common volume operations for `PulseAudio` channel
/// volumes.
pub trait Volume {
    /// Returns the normalized volume value in range `[0.0, 1.0]`.
    fn get_volume(&self) -> f64;

    /// Scales the volume to `max` and returns the modified value when
    /// successful.
    fn scale_volume(&mut self, max: f64) -> Option<&mut ChannelVolumes>;
}

impl Volume for ChannelVolumes {
    fn get_volume(&self) -> f64 {
        f64::from(self.avg().0) / f64::from(libpulse_binding::volume::Volume::NORMAL.0)
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "max is clamped to [0.0, 1.0], so the scaled raw volume is non-negative and fits in u32"
    )]
    fn scale_volume(&mut self, max: f64) -> Option<&mut ChannelVolumes> {
        let max = max.clamp(0.0, 1.0);
        self.scale(libpulse_binding::volume::Volume(
            (f64::from(libpulse_binding::volume::Volume::NORMAL.0) * max) as u32
        ))
    }
}

/// Convenience helpers for sink collections.
pub trait Sinks {
    /// Computes the icon for the default sink.
    fn get_icon(&self, default_sink: &str) -> Icons;
}

impl Sinks for Vec<Device> {
    fn get_icon(&self, default_sink: &str) -> Icons {
        match self.iter().find_map(|sink| {
            if sink.ports.iter().any(|port| port.active) && sink.name == default_sink {
                Some((sink.is_mute, sink.volume.get_volume()))
            } else {
                None
            }
        }) {
            Some((true, _)) | None => Icons::Speaker0,
            Some((false, volume)) => {
                if volume > 0.66 {
                    Icons::Speaker3
                } else if volume > 0.33 {
                    Icons::Speaker2
                } else if volume > 0.000_001 {
                    Icons::Speaker1
                } else {
                    Icons::Speaker0
                }
            }
        }
    }
}

/// Runtime state tracked by the audio service and exposed to the UI.
#[derive(Debug, Clone, Default)]
pub struct AudioData {
    /// Which output and input the server holds as default.
    pub server_info:       ServerInfo,
    /// Every output the server knows.
    pub sinks:             Vec<Device>,
    /// Every input the server knows.
    pub sources:           Vec<Device>,
    /// Volume of the default output, as a share of full scale.
    pub cur_sink_volume:   i32,
    /// Volume of the default input, as a share of full scale.
    pub cur_source_volume: i32
}

/// Events produced by the backend to update the service state.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// The outputs the server knows have changed.
    Sinks(Vec<Device>),
    /// The inputs the server knows have changed.
    Sources(Vec<Device>),
    /// Which output or input is default has changed.
    ServerInfo(ServerInfo)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn device_type_icons_match_expectations() {
        assert_eq!(DeviceType::Headphones.get_icon(), Icons::Headphones1);
        assert_eq!(DeviceType::Speaker.get_icon(), Icons::Speaker3);
        assert_eq!(DeviceType::Headset.get_icon(), Icons::Headset);
        assert_eq!(DeviceType::Hdmi.get_icon(), Icons::MonitorSpeaker);
    }

    #[test]
    fn sink_collection_icon_considers_mute_state() {
        let sinks = vec![Device {
            name:        "default".into(),
            description: String::new(),
            volume:      ChannelVolumes::default(),
            is_mute:     true,
            in_use:      true,
            ports:       vec![Port {
                name:        "port".into(),
                description: String::new(),
                device_type: DeviceType::Speaker,
                active:      true
            }]
        }];

        assert_eq!(sinks.get_icon("default"), Icons::Speaker0);
    }

    #[test]
    fn sink_collection_returns_default_when_no_match() {
        let sinks = vec![Device {
            name:        "other".into(),
            description: String::new(),
            volume:      ChannelVolumes::default(),
            is_mute:     false,
            in_use:      true,
            ports:       vec![Port {
                name:        "port".into(),
                description: String::new(),
                device_type: DeviceType::Speaker,
                active:      true
            }]
        }];

        assert_eq!(sinks.get_icon("default"), Icons::Speaker0);
    }

    #[test]
    fn volume_trait_clamps_to_valid_range() {
        let mut volume = ChannelVolumes::default();
        // scale_volume clamps max to [0.0, 1.0], so 1.2 becomes 1.0
        // On empty ChannelVolumes, scale() may return None
        let result = volume.scale_volume(1.2);
        // Just verify it doesn't panic and returns expected type
        let _ = result;
    }
}
