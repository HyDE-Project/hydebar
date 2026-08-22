#![allow(mismatched_lifetime_syntaxes)]

mod agents;
mod backend;
mod events;
mod proxies;
mod queries;
mod station_state;

#[cfg(test)]
mod tests;

pub mod access_point;
pub mod adapter;
pub mod agent_manager;
pub mod basic_service_set;
pub mod daemon;
pub mod device;
pub mod device_provisioning;
pub mod known_network;
pub mod network;
pub mod service_manager;
pub mod shared_code_device_provisioning;
pub mod simple_configuration;
pub mod station;
pub mod station_diagnostic;

use std::ops::Deref;

use zbus::fdo::ObjectManagerProxy;

// source for dbus: https://git.kernel.org/pub/scm/network/wireless/iwd.git/tree/doc
//info!("{:?}",n.inner().introspect().await?); => can use this to generate
// proxy implementations

/// Wrapper around the IWD D-Bus `ObjectManager`
pub struct IwdDbus<'a> {
    inner: ObjectManagerProxy<'a>
}

impl std::fmt::Debug for IwdDbus<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IwdDbus").finish_non_exhaustive()
    }
}

impl<'a> Deref for IwdDbus<'a> {
    type Target = ObjectManagerProxy<'a>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
