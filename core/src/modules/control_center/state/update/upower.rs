//! Handling of power profile messages from the `UPower` service.

use super::super::super::{
    ControlCenter, commands::ControlCenterCommandExt, upower::UPowerMessage
};
use crate::services::{ReadOnlyService, ServiceEvent, upower::PowerProfileCommand};

impl ControlCenter {
    pub(super) fn handle_upower(&mut self, msg: UPowerMessage) {
        match msg {
            UPowerMessage::Event(event) => match *event {
                ServiceEvent::Init(service) => {
                    self.upower = Some(service);
                }
                ServiceEvent::Update(data) => {
                    if let Some(upower) = self.upower.as_mut() {
                        upower.update(data);
                    }
                }
                ServiceEvent::Error(err) => {
                    log::error!("UPower service error: {err:?}");
                }
            },
            UPowerMessage::TogglePowerProfile => {
                let _spawned = self.spawn_upower_command(PowerProfileCommand::Toggle);
            }
        }
    }
}
