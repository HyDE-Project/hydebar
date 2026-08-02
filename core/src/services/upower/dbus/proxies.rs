//! Proxy definitions for the `UPower` D-Bus interfaces.

use zbus::{Result, proxy, zvariant::OwnedObjectPath};

#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
pub trait UPower {
    fn enumerate_devices(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(signal)]
    fn device_added(&self) -> Result<OwnedObjectPath>;
}

#[proxy(
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/Device",
    interface = "org.freedesktop.UPower.Device"
)]
pub trait Device {
    #[zbus(property, name = "Type")]
    fn device_type(&self) -> Result<u32>;

    #[zbus(property)]
    fn power_supply(&self) -> Result<bool>;

    #[zbus(property)]
    fn time_to_empty(&self) -> Result<i64>;

    #[zbus(property)]
    fn time_to_full(&self) -> Result<i64>;

    #[zbus(property)]
    fn percentage(&self) -> Result<f64>;

    #[zbus(property)]
    fn state(&self) -> Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.UPower.PowerProfiles",
    default_path = "/org/freedesktop/UPower/PowerProfiles",
    interface = "org.freedesktop.UPower.PowerProfiles"
)]
pub trait PowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> Result<String>;

    #[zbus(property)]
    fn set_active_profile(&self, profile: &str) -> Result<()>;
}
