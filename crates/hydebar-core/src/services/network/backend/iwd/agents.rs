//! Station state and the D-Bus agents iwd calls back into.

use log::warn;
use zbus::{interface, zvariant::OwnedObjectPath};

pub(super) enum IwdStationState {
    Connected,
    Disconnected,
    Connecting,
    Disconnecting,
    Roaming
}

impl From<String> for IwdStationState {
    fn from(state: String) -> Self {
        match state.as_str() {
            "connected" => IwdStationState::Connected,
            "disconnected" => IwdStationState::Disconnected,
            "connecting" => IwdStationState::Connecting,
            "disconnecting" => IwdStationState::Disconnecting,
            "roaming" => IwdStationState::Roaming,
            _ => IwdStationState::Disconnected
        }
    }
}

pub(super) struct SignalAgent {
    pub(super) tx: tokio::sync::mpsc::UnboundedSender<i16>
}

#[interface(name = "net.connman.iwd.SignalLevelAgent")]
impl SignalAgent {
    /// Called by iwd whenever RSSI crosses a threshold
    #[zbus(name = "Changed")]
    pub(super) fn changed(&self, level: i16) {
        // ignore failure if receiver was dropped
        warn!("Signal level changed: {level}");
        let _ = self.tx.send(level);
    }
}

pub(super) struct PWAgent {
    // Channel for receiving passwords
    pub(super) password_rx: tokio::sync::mpsc::UnboundedReceiver<String>
}

#[interface(name = "net.connman.iwd.Agent")]
impl PWAgent {
    #[zbus(name = "RequestPassphrase")]
    pub(super) async fn request_passphrase(
        &mut self,
        _network_path: OwnedObjectPath
    ) -> zbus::fdo::Result<String> {
        // Try to receive a password from the channel
        if let Ok(pass) = self.password_rx.try_recv() {
            Ok(pass)
        } else {
            warn!("No password available");
            Err(zbus::fdo::Error::Failed("No password set".into()))
        }
    }
}
