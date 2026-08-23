use std::any::TypeId;

use iced::Subscription;
use zbus::zvariant::ObjectPath;

use super::{ReadOnlyService, ServiceEvent};

mod dbus;

mod command;
mod events;
mod init;
mod model;

pub use command::PowerProfileCommand;
pub use model::{BatteryData, BatteryStatus, PowerProfile, UPowerEvent};

/// The conversation with the power daemon.
#[derive(Debug, Clone)]
pub struct UPowerService {
    /// The battery, where the machine has one.
    pub battery:       Option<BatteryData>,
    /// The power profile in force.
    pub power_profile: PowerProfile,
    conn:              zbus::Connection
}

pub(crate) enum State {
    Init,
    Active(zbus::Connection, Option<Vec<ObjectPath<'static>>>),
    Error
}

impl ReadOnlyService for UPowerService {
    type UpdateEvent = UPowerEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            UPowerEvent::UpdateBattery(data) => {
                self.battery.replace(data);
            }
            UPowerEvent::NoBattery => {
                self.battery = None;
            }
            UPowerEvent::UpdatePowerProfile(profile) => {
                self.power_profile = profile;
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Self::subscription_with_id(TypeId::of::<Self>())
    }
}
