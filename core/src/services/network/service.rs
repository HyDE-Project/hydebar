use std::ops::Deref;

pub use super::data::{
    AccessPoint, ActiveConnectionInfo, ConnectivityState, DeviceState, KnownConnection,
    NetworkCommand, NetworkData, NetworkEvent, NetworkServiceError, Vpn
};

#[derive(Debug, Clone)]
/// Reactive service responsible for keeping track of the system network state.
///
/// # Examples
/// ```no_run
/// # async fn demo(service: &mut hydebar_core::services::network::NetworkService) {
/// use hydebar_core::services::network::NetworkServiceError;
/// service.apply_error(NetworkServiceError::new("temporary failure"));
/// # }
/// ```
pub struct NetworkService {
    data:           NetworkData,
    conn:           zbus::Connection,
    backend_choice: BackendChoice
}

impl Deref for NetworkService {
    type Target = NetworkData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

enum State {
    Init,
    Active(zbus::Connection, BackendChoice),
    Error
}

mod choice;
mod command;
pub use command::AccessPointProbe;
mod gate;
mod listen;
mod reactive;

#[cfg(test)]
mod tests;

use choice::BackendChoice;
