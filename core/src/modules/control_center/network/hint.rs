//! The hover text of the network entry, one connection fact per line.

use super::super::ControlCenter;
use crate::services::network::{ActiveConnectionInfo, NetworkData};

impl ControlCenter {
    /// One-look summary of the connection, for the pointer resting on the
    /// network module.
    #[must_use]
    pub fn network_hint(&self) -> Option<String> {
        self.network
            .as_ref()
            .map(|service| service.connection_hint())
    }
}

impl NetworkData {
    /// States the connection the way its hover reads: the network, the
    /// signal, the frequency, the interface, the addressing — every fact
    /// the bar holds, one per line — or the one word explaining why
    /// there is nothing to state.
    #[must_use]
    pub fn connection_hint(&self) -> String {
        let mut lines = Vec::new();
        let mut vpns = Vec::new();

        for connection in &self.active_connections {
            match connection {
                ActiveConnectionInfo::WiFi {
                    name,
                    strength,
                    ..
                } => {
                    lines.push(format!("Network: {name}"));
                    lines.push(self.link.signal_dbm.map_or_else(
                        || format!("Signal strength: {strength}%"),
                        |dbm| format!("Signal strength: {dbm}dBm ({strength}%)")
                    ));

                    if let Some(mhz) = self.link.frequency_mhz {
                        lines.push(format!("Frequency: {mhz}MHz"));
                    }
                }
                ActiveConnectionInfo::Wired {
                    name,
                    speed
                } => {
                    lines.push(format!("Wired: {name}"));

                    if *speed > 0 {
                        lines.push(format!("Speed: {speed} Mb/s"));
                    }
                }
                ActiveConnectionInfo::Vpn {
                    name, ..
                } => vpns.push(name.clone())
            }
        }

        if !lines.is_empty() {
            if let Some(interface) = &self.link.interface {
                lines.push(format!("Interface: {interface}"));
            }

            if let Some(address) = &self.link.address {
                lines.push(format!("IP: {address}"));
            }

            if let Some(gateway) = &self.link.gateway {
                lines.push(format!("Gateway: {gateway}"));
            }

            if let Some(netmask) = &self.link.netmask {
                lines.push(format!("Netmask: {netmask}"));
            }
        }

        for vpn in vpns {
            lines.push(format!("VPN: {vpn}"));
        }

        if lines.is_empty() {
            lines.push(
                if self.airplane_mode {
                    "Airplane mode"
                } else if self.wifi_present && !self.wifi_enabled {
                    "Wi-Fi off"
                } else {
                    "Disconnected"
                }
                .to_owned()
            );
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hover_states_the_wifi_and_its_strength() {
        let data = NetworkData {
            active_connections: vec![ActiveConnectionInfo::WiFi {
                id:       "home".to_owned(),
                name:     "HomeNet".to_owned(),
                strength: 87
            }],
            ..NetworkData::default()
        };

        assert_eq!(
            data.connection_hint(),
            "Network: HomeNet\nSignal strength: 87%"
        );
    }

    #[test]
    fn the_hover_states_every_fact_of_the_link_it_holds() {
        let data = NetworkData {
            active_connections: vec![ActiveConnectionInfo::WiFi {
                id:       "home".to_owned(),
                name:     "HomeNet".to_owned(),
                strength: 87
            }],
            link: crate::services::network::LinkDetails {
                interface:     Some("wlan0".to_owned()),
                signal_dbm:    Some(-27),
                frequency_mhz: Some(5320),
                address:       Some("192.168.2.19/24".to_owned()),
                gateway:       Some("192.168.2.253".to_owned()),
                netmask:       Some("255.255.255.0".to_owned())
            },
            ..NetworkData::default()
        };

        assert_eq!(
            data.connection_hint(),
            "Network: HomeNet\nSignal strength: -27dBm (87%)\nFrequency: \
         5320MHz\nInterface: wlan0\nIP: 192.168.2.19/24\nGateway: \
         192.168.2.253\nNetmask: 255.255.255.0"
        );
    }

    #[test]
    fn the_hover_states_the_wire_and_any_vpn_on_top() {
        let data = NetworkData {
            active_connections: vec![
                ActiveConnectionInfo::Wired {
                    name:  "eth0".to_owned(),
                    speed: 1000
                },
                ActiveConnectionInfo::Vpn {
                    name:        "work".to_owned(),
                    object_path: zbus::zvariant::OwnedObjectPath::try_from("/").expect("path")
                },
            ],
            ..NetworkData::default()
        };

        assert_eq!(
            data.connection_hint(),
            "Wired: eth0\nSpeed: 1000 Mb/s\nVPN: work"
        );
    }

    #[test]
    fn a_wire_without_a_reported_speed_keeps_that_line_to_itself() {
        let data = NetworkData {
            active_connections: vec![ActiveConnectionInfo::Wired {
                name:  "eth0".to_owned(),
                speed: 0
            }],
            ..NetworkData::default()
        };

        assert_eq!(data.connection_hint(), "Wired: eth0");
    }

    #[test]
    fn nothing_connected_names_the_reason() {
        assert_eq!(NetworkData::default().connection_hint(), "Disconnected");

        let airplane = NetworkData {
            airplane_mode: true,
            ..NetworkData::default()
        };
        assert_eq!(airplane.connection_hint(), "Airplane mode");

        let radio_off = NetworkData {
            wifi_present: true,
            wifi_enabled: false,
            ..NetworkData::default()
        };
        assert_eq!(radio_off.connection_hint(), "Wi-Fi off");
    }
}
