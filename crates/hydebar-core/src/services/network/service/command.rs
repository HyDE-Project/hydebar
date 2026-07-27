//! Execution of user issued network commands.

use iced::Task;
use log::debug;

use super::super::{
    ActiveConnectionInfo, NetworkCommand, NetworkEvent, NetworkService, backend::NetworkBackend
};
use crate::services::{Service, ServiceEvent};

impl NetworkService {
    pub async fn run_command(self, command: NetworkCommand) -> ServiceEvent<Self> {
        let mut bc = self.backend_choice.with_connection(self.conn.clone());

        match command {
            NetworkCommand::ToggleAirplaneMode => {
                let airplane_mode = self.airplane_mode;
                debug!("Toggling airplane mode to: {}", !airplane_mode);
                let result = bc.set_airplane_mode(!airplane_mode).await;
                let new_state = if result.is_ok() {
                    !airplane_mode
                } else {
                    airplane_mode
                };

                ServiceEvent::Update(NetworkEvent::AirplaneMode(new_state))
            }
            NetworkCommand::ScanNearByWiFi => {
                let _ = bc.scan_nearby_wifi().await;
                ServiceEvent::Update(NetworkEvent::ScanningNearbyWifi)
            }
            NetworkCommand::ToggleWiFi => {
                let wifi_enabled = self.wifi_enabled;
                debug!("Toggling wifi to: {}", !wifi_enabled);
                let result = bc.set_wifi_enabled(!wifi_enabled).await;
                let new_state = if result.is_ok() {
                    !wifi_enabled
                } else {
                    wifi_enabled
                };

                ServiceEvent::Update(NetworkEvent::WiFiEnabled(new_state))
            }
            NetworkCommand::SelectAccessPoint((access_point, password)) => {
                bc.select_access_point(&access_point, password)
                    .await
                    .unwrap_or_default();
                let known_connections = bc.known_connections().await.unwrap_or_default();

                ServiceEvent::Update(NetworkEvent::KnownConnections(known_connections))
            }
            NetworkCommand::ToggleVpn(vpn) => {
                let mut active_vpn = self.active_connections.iter().find_map(|kc| match kc {
                    ActiveConnectionInfo::Vpn {
                        name,
                        object_path
                    } if name == &vpn.name => Some(object_path.clone()),
                    _ => None
                });

                let (object_path, new_state) = if let Some(active_vpn) = active_vpn.take() {
                    (active_vpn, false)
                } else {
                    (vpn.path, true)
                };

                bc.set_vpn(object_path, new_state).await.unwrap_or_default();
                let known_connections = bc.known_connections().await.unwrap_or_default();

                ServiceEvent::Update(NetworkEvent::KnownConnections(known_connections))
            }
        }
    }
}

impl Service for NetworkService {
    type Command = NetworkCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        debug!("Command: {command:?}");
        let service = self.clone();

        Task::perform(
            async move { NetworkService::run_command(service, command).await },
            |event| event
        )
    }
}
