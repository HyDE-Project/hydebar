//! Data model shared by the network service and its backends.

mod error;
mod event;
mod model;

pub use error::NetworkServiceError;
pub use event::{NetworkCommand, NetworkEvent};
pub use model::{
    AccessPoint, ActiveConnectionInfo, ConnectivityState, DeviceState, KnownConnection,
    LinkDetails, NetworkData, Vpn
};
