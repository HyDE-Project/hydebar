//! Translation of bus events into application messages.

use hydebar_core::event_bus::{BusEvent, ModuleEvent};

use super::super::state::{App, Message};

impl App {
    /// Maps a bus event onto the message that handles it.
    ///
    /// Total on purpose: the enums are workspace-local and every new
    /// variant must be named here, so an event can never be dropped
    /// silently at the door.
    pub(super) fn message_from_bus_event(event: BusEvent) -> Message {
        match event {
            BusEvent::Redraw => Message::None,
            BusEvent::PopupToggle => Message::CloseAllMenus,
            BusEvent::Module(module) => Self::message_from_module_event(module)
        }
    }

    pub(super) fn message_from_module_event(event: ModuleEvent) -> Message {
        match event {
            ModuleEvent::Updates(message) => Message::Updates(message),
            ModuleEvent::Workspaces(message) => Message::Workspaces(message),
            ModuleEvent::WindowTitle(message) => Message::WindowTitle(message),
            ModuleEvent::SystemInfo(message) => Message::SystemInfo(message),
            ModuleEvent::KeyboardLayout(message) => Message::KeyboardLayout(message),
            ModuleEvent::KeyboardSubmap(message) => Message::KeyboardSubmap(message),
            ModuleEvent::Tray(message) => Message::Tray(message),
            ModuleEvent::Clock(message) => Message::Clock(message),
            ModuleEvent::Weather(message) => Message::Weather(message),
            ModuleEvent::Battery(message) => Message::Battery(message),
            ModuleEvent::Privacy(message) => Message::Privacy(message),
            ModuleEvent::ControlCenter(message) => Message::ControlCenter(message),
            ModuleEvent::MediaPlayer(message) => Message::MediaPlayer(message),
            ModuleEvent::Notifications(message) => Message::Notifications(message),
            ModuleEvent::Custom {
                name,
                message
            } => Message::CustomUpdate(name, message)
        }
    }
}
