//! The D-Bus agents iwd calls back into.

use log::warn;
use zbus::{interface, zvariant::OwnedObjectPath};

pub(super) struct SignalAgent {
    pub(super) tx: tokio::sync::mpsc::UnboundedSender<i16>
}

#[interface(name = "net.connman.iwd.SignalLevelAgent")]
impl SignalAgent {
    /// Called by iwd whenever RSSI crosses a threshold.
    ///
    /// A send failure only means the receiver was dropped, so it is ignored.
    #[zbus(name = "Changed")]
    pub(super) fn changed(&self, level: i16) {
        warn!("Signal level changed: {level}");
        let _ = self.tx.send(level);
    }
}

pub(super) struct PWAgent {
    /// Channel the requested passwords arrive on.
    pub(super) password_rx: tokio::sync::mpsc::UnboundedReceiver<String>
}

#[interface(name = "net.connman.iwd.Agent")]
impl PWAgent {
    #[zbus(name = "RequestPassphrase")]
    #[expect(
        clippy::unused_async,
        reason = "the zbus interface macro exposes this handler as an async D-Bus method"
    )]
    pub(super) async fn request_passphrase(
        &mut self,
        network_path: OwnedObjectPath
    ) -> zbus::fdo::Result<String> {
        let _ = network_path;
        self.password_rx.try_recv().map_err(|_| {
            warn!("No password available");
            zbus::fdo::Error::Failed("No password set".into())
        })
    }
}
