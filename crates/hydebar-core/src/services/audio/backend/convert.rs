//! Conversion of `PulseAudio` structures into the service model.

use libpulse_binding::{
    context::introspect::{SinkInfo, SourceInfo},
    def::{DevicePortType, PortAvailable, SinkState, SourceState}
};

use crate::services::audio::model::{Device, DeviceType, Port, ServerInfo};

impl From<&libpulse_binding::context::introspect::ServerInfo<'_>> for ServerInfo {
    fn from(value: &libpulse_binding::context::introspect::ServerInfo<'_>) -> Self {
        Self {
            default_sink:   value
                .default_sink_name
                .as_ref()
                .map_or_else(String::default, ToString::to_string),
            default_source: value
                .default_source_name
                .as_ref()
                .map_or_else(String::default, ToString::to_string)
        }
    }
}

impl From<&SinkInfo<'_>> for Device {
    fn from(value: &SinkInfo<'_>) -> Self {
        Self {
            name:        value
                .name
                .as_ref()
                .map_or(String::default(), ToString::to_string),
            description: value
                .proplist
                .get_str("device.description")
                .unwrap_or_default(),
            volume:      value.volume,
            is_mute:     value.mute,
            in_use:      value.state == SinkState::Running,
            ports:       value
                .ports
                .iter()
                .filter_map(|port| {
                    if port.available == PortAvailable::No {
                        None
                    } else {
                        Some(Port {
                            name:        port
                                .name
                                .as_ref()
                                .map_or(String::default(), ToString::to_string),
                            description: port
                                .description
                                .as_ref()
                                .map_or(String::default(), ToString::to_string),
                            device_type: match port.r#type {
                                DevicePortType::Headphones => DeviceType::Headphones,
                                DevicePortType::Speaker => DeviceType::Speaker,
                                DevicePortType::Headset => DeviceType::Headset,
                                DevicePortType::HDMI => DeviceType::Hdmi,
                                _ => DeviceType::Speaker
                            },
                            active:      value.active_port.as_ref().and_then(|p| p.name.as_ref())
                                == port.name.as_ref()
                        })
                    }
                })
                .collect::<Vec<_>>()
        }
    }
}

impl From<&SourceInfo<'_>> for Device {
    fn from(value: &SourceInfo<'_>) -> Self {
        Self {
            name:        value
                .name
                .as_ref()
                .map_or(String::default(), ToString::to_string),
            description: value
                .proplist
                .get_str("device.description")
                .unwrap_or_default(),
            volume:      value.volume,
            is_mute:     value.mute,
            in_use:      value.state == SourceState::Running,
            ports:       value
                .ports
                .iter()
                .filter_map(|port| {
                    if port.available == PortAvailable::No {
                        None
                    } else {
                        Some(Port {
                            name:        port
                                .name
                                .as_ref()
                                .map_or(String::default(), ToString::to_string),
                            description: port
                                .description
                                .as_ref()
                                .map_or(String::default(), ToString::to_string),
                            device_type: match port.r#type {
                                DevicePortType::Headphones => DeviceType::Headphones,
                                DevicePortType::Speaker => DeviceType::Speaker,
                                DevicePortType::Headset => DeviceType::Headset,
                                DevicePortType::HDMI => DeviceType::Hdmi,
                                _ => DeviceType::Speaker
                            },
                            active:      value.active_port.as_ref().and_then(|p| p.name.as_ref())
                                == port.name.as_ref()
                        })
                    }
                })
                .collect::<Vec<_>>()
        }
    }
}
