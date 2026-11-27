//! The power state the bar reads and commands.

use super::model::{BatteryData, PowerProfile};

/// The conversation with the power daemon.
#[derive(Debug, Clone)]
pub struct UPowerService {
    /// The battery, where the machine has one.
    pub battery:       Option<BatteryData>,
    /// The power profile in force.
    pub power_profile: PowerProfile,
    pub(super) conn:   zbus::Connection
}
