//! Reactive service surface consumed by the bar modules.

use std::any::TypeId;

use iced::{Subscription, stream::channel};
use log::debug;

use super::super::{ActiveConnectionInfo, NetworkEvent, NetworkService, NetworkServiceError};
use crate::services::{ReadOnlyService, ServiceEvent};

impl ReadOnlyService for NetworkService {
    type UpdateEvent = NetworkEvent;
    type Error = NetworkServiceError;

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data.last_error = None;
        match event {
            NetworkEvent::AirplaneMode(airplane_mode) => {
                self.data.airplane_mode = airplane_mode;
            }
            NetworkEvent::WiFiEnabled(wifi_enabled) => {
                debug!("WiFi enabled: {wifi_enabled}");
                self.data.wifi_enabled = wifi_enabled;
            }
            NetworkEvent::ScanningNearbyWifi => {
                self.data.scanning_nearby_wifi = true;
            }
            NetworkEvent::WirelessDevice {
                wifi_present,
                wireless_access_points
            } => {
                self.data.wifi_present = wifi_present;
                self.data.scanning_nearby_wifi = false;
                self.data.wireless_access_points = wireless_access_points;
            }
            NetworkEvent::ActiveConnections(active_connections) => {
                self.data.active_connections = active_connections;
            }
            NetworkEvent::KnownConnections(known_connections) => {
                self.data.known_connections = known_connections;
            }
            NetworkEvent::Strength((ssid, new_strength)) => {
                if let Some(ap) = self
                    .data
                    .wireless_access_points
                    .iter_mut()
                    .find(|ap| ap.ssid == ssid)
                {
                    ap.strength = new_strength;
                }

                if let Some(ActiveConnectionInfo::WiFi {
                    strength, ..
                }) = self
                    .data
                    .active_connections
                    .iter_mut()
                    .find(|ac| ac.name() == ssid)
                {
                    *strength = new_strength;
                }
            }
            NetworkEvent::Connectivity(connectivity) => {
                self.data.connectivity = connectivity;
            }
            NetworkEvent::WirelessAccessPoint(wireless_access_points) => {
                self.data.wireless_access_points = wireless_access_points;
            }
            NetworkEvent::LinkDetails(link) => {
                self.data.link = link;
            }
            NetworkEvent::RequestPasswordForSSID(_) => {}
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with(id, |&_id| {
            channel(50, async |mut output| {
                Self::listen(&mut output).await;
            })
        })
    }
}
