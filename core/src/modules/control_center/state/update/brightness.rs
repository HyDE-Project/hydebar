//! Handling of backlight messages: service events and level changes.

use super::super::super::{
    ControlCenter, brightness::BrightnessMessage, commands::ControlCenterCommandExt
};
use crate::services::{ReadOnlyService, ServiceEvent, brightness::BrightnessCommand};

impl ControlCenter {
    pub(super) fn handle_brightness(&mut self, msg: BrightnessMessage) {
        match msg {
            BrightnessMessage::Event(event) => match *event {
                ServiceEvent::Init(service) => {
                    self.brightness = Some(service);
                }
                ServiceEvent::Update(data) => {
                    if let Some(brightness) = self.brightness.as_mut() {
                        brightness.update(data);
                    }
                }
                ServiceEvent::Error(err) => {
                    log::error!("Brightness service error: {err:?}");
                }
            },
            BrightnessMessage::Change(value) => {
                let _spawned = self.spawn_brightness_command(BrightnessCommand::Set(value));
            }
        }
    }
}
