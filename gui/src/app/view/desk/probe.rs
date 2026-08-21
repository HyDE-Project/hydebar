use hydebar_core::services::{
    network::ActiveConnectionInfo,
    upower::PowerProfile
};

use super::super::super::state::App;

#[expect(dead_code, reason = "probe")]
impl App {
    fn cc_idle(&self) -> String {
        if self.control_center.is_idle_inhibited() {
            "held awake".to_owned()
        } else {
            "allowed".to_owned()
        }
    }

    fn cc_hint(&self) -> Option<String> {
        self.control_center.network_hint()
    }

    fn a_sink(&self) -> Option<String> {
        self.control_center.audio_data().and_then(|audio| {
            audio
                .sinks
                .iter()
                .find(|sink| sink.name == audio.server_info.default_sink)
                .map(|sink| sink.description.clone())
        })
    }

    fn a_volume(&self) -> Option<String> {
        self.control_center.audio_data().map(|audio| {
            let muted = audio
                .sinks
                .iter()
                .find(|sink| sink.name == audio.server_info.default_sink)
                .is_some_and(|sink| sink.is_mute);

            if muted {
                format!("muted ({}%)", audio.cur_sink_volume)
            } else {
                format!("{}%", audio.cur_sink_volume)
            }
        })
    }

    fn a_source(&self) -> Option<String> {
        self.control_center.audio_data().and_then(|audio| {
            audio
                .sources
                .iter()
                .find(|source| source.name == audio.server_info.default_source)
                .map(|source| source.description.clone())
        })
    }

    fn a_source_volume(&self) -> Option<String> {
        self.control_center.audio_data().map(|audio| {
            let muted = audio
                .sources
                .iter()
                .find(|source| source.name == audio.server_info.default_source)
                .is_some_and(|source| source.is_mute);

            if muted {
                format!("muted ({}%)", audio.cur_source_volume)
            } else {
                format!("{}%", audio.cur_source_volume)
            }
        })
    }

    fn a_sink_count(&self) -> Option<String> {
        self.control_center
            .audio_data()
            .map(|audio| audio.sinks.len().to_string())
    }

    fn n_wifi(&self) -> Option<String> {
        self.control_center.network_data().map(|network| {
            if !network.wifi_present {
                "absent".to_owned()
            } else if network.airplane_mode {
                "airplane mode".to_owned()
            } else if network.wifi_enabled {
                "on".to_owned()
            } else {
                "off".to_owned()
            }
        })
    }

    fn n_ssid(&self) -> Option<String> {
        self.control_center.network_data().and_then(|network| {
            network.active_connections.iter().find_map(|connection| {
                match connection {
                    ActiveConnectionInfo::WiFi {
                        name,
                        strength,
                        ..
                    } => Some(format!("{name} ({strength}%)")),
                    _ => None
                }
            })
        })
    }

    fn n_wired(&self) -> Option<String> {
        self.control_center.network_data().and_then(|network| {
            network.active_connections.iter().find_map(|connection| {
                match connection {
                    ActiveConnectionInfo::Wired {
                        name,
                        speed
                    } if *speed > 0 => Some(format!("{name} at {speed} Mb/s")),
                    ActiveConnectionInfo::Wired {
                        name, ..
                    } => Some(name.clone()),
                    _ => None
                }
            })
        })
    }

    fn n_vpn(&self) -> Option<String> {
        self.control_center.network_data().and_then(|network| {
            let names: Vec<String> = network
                .active_connections
                .iter()
                .filter_map(|connection| match connection {
                    ActiveConnectionInfo::Vpn {
                        name, ..
                    } => Some(name.clone()),
                    _ => None
                })
                .collect();

            (!names.is_empty()).then(|| names.join(", "))
        })
    }

    fn n_address(&self) -> Option<String> {
        self.control_center
            .network_data()
            .and_then(|network| network.link.address.clone())
    }

    fn n_gateway(&self) -> Option<String> {
        self.control_center
            .network_data()
            .and_then(|network| network.link.gateway.clone())
    }

    fn n_interface(&self) -> Option<String> {
        self.control_center
            .network_data()
            .and_then(|network| network.link.interface.clone())
    }

    fn n_signal(&self) -> Option<String> {
        self.control_center
            .network_data()
            .and_then(|network| network.link.signal_dbm)
            .map(|dbm| format!("{dbm} dBm"))
    }

    fn n_band(&self) -> Option<String> {
        self.control_center
            .network_data()
            .and_then(|network| network.link.frequency_mhz)
            .map(|mhz| format!("{mhz} MHz"))
    }

    fn n_nearby(&self) -> Option<String> {
        self.control_center
            .network_data()
            .map(|network| network.wireless_access_points.len().to_string())
    }

    fn b_radio(&self) -> Option<String> {
        use hydebar_core::services::bluetooth::BluetoothState;

        self.control_center
            .bluetooth_data()
            .map(|bluetooth| match bluetooth.state {
                BluetoothState::Active => "on".to_owned(),
                BluetoothState::Inactive => "off".to_owned(),
                BluetoothState::Unavailable => "absent".to_owned()
            })
    }

    fn b_connected(&self) -> Option<String> {
        self.control_center.bluetooth_data().and_then(|bluetooth| {
            let names: Vec<String> = bluetooth
                .devices
                .iter()
                .filter(|device| device.connected)
                .map(|device| match device.battery {
                    Some(battery) => format!("{} ({battery}%)", device.name),
                    None => device.name.clone()
                })
                .collect();

            (!names.is_empty()).then(|| names.join(", "))
        })
    }

    fn b_paired(&self) -> Option<String> {
        self.control_center
            .bluetooth_data()
            .map(|bluetooth| bluetooth.devices.len().to_string())
    }

    fn br_percent(&self) -> Option<String> {
        self.control_center.brightness_data().and_then(|brightness| {
            (brightness.max > 0).then(|| {
                format!(
                    "{}%",
                    (u64::from(brightness.current) * 100 / u64::from(brightness.max))
                )
            })
        })
    }

    fn pp_profile(&self) -> Option<String> {
        self.control_center
            .power_profile()
            .map(|profile| match profile {
                PowerProfile::Balanced => "balanced".to_owned(),
                PowerProfile::Performance => "performance".to_owned(),
                PowerProfile::PowerSaver => "power saver".to_owned(),
                PowerProfile::Unknown => "unknown".to_owned()
            })
    }
}
