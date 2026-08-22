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

    /// Share of the design charge the cell can still hold.
    ///
    /// A battery wears out, and a machine that reports ninety percent of a
    /// cell that holds seventy of what it was sold with is not the machine it
    /// was. Nothing else on the bar says so.
    #[zbus(property)]
    fn capacity(&self) -> Result<f64>;

    /// How many times the cell has been charged through.
    #[zbus(property)]
    fn charge_cycles(&self) -> Result<i32>;

    /// What the cell is giving or taking right now, in watts.
    #[zbus(property, name = "EnergyRate")]
    fn energy_rate(&self) -> Result<f64>;

    /// What the cell holds now, in watt hours.
    #[zbus(property)]
    fn energy(&self) -> Result<f64>;

    /// What the cell holds when it is full, in watt hours.
    #[zbus(property, name = "EnergyFull")]
    fn energy_full(&self) -> Result<f64>;
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
