pub use super::data::{
    AccessPoint, ActiveConnectionInfo, ConnectivityState, DeviceState, KnownConnection,
    NetworkCommand, NetworkData, NetworkEvent, NetworkServiceError, Vpn
};

enum State {
    Init,
    Active(zbus::Connection, BackendChoice),
    Error
}

mod choice;
mod command;
mod handle;
pub use command::AccessPointProbe;
pub use handle::NetworkService;
mod gate;
mod listen;
mod reactive;

#[cfg(test)]
mod tests;

use choice::BackendChoice;
