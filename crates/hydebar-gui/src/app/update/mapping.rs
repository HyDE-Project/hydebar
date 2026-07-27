//! Translation of bus events into application messages.

use hydebar_core::event_bus::{BusEvent, ModuleEvent};

use super::super::state::{App, Message};

impl App {
    /// Frame clock feeding the animators.
    ///
    /// The subscription exists only while something is animating, so an idle
    /// panel stops asking the compositor for frame callbacks entirely instead

    pub(super) fn message_from_bus_event(event: BusEvent) -> Option<Message> {
        match event {
            BusEvent::Redraw => Some(Message::None),
            BusEvent::PopupToggle => Some(Message::CloseAllMenus),
            BusEvent::Module(module) => App::message_from_module_event(module),
            _ => None
        }
    }

    pub(super) fn message_from_module_event(event: ModuleEvent) -> Option<Message> {
        match event {
            ModuleEvent::Updates(message) => Some(Message::Updates(message)),
            ModuleEvent::Workspaces(message) => Some(Message::Workspaces(message)),
            ModuleEvent::WindowTitle(message) => Some(Message::WindowTitle(message)),
            ModuleEvent::SystemInfo(message) => Some(Message::SystemInfo(message)),
            ModuleEvent::KeyboardLayout(message) => Some(Message::KeyboardLayout(message)),
            ModuleEvent::KeyboardSubmap(message) => Some(Message::KeyboardSubmap(message)),
            ModuleEvent::Tray(message) => Some(Message::Tray(message)),
            ModuleEvent::Clock(message) => Some(Message::Clock(message)),
            ModuleEvent::Weather(message) => Some(Message::Weather(message)),
            ModuleEvent::Battery(message) => Some(Message::Battery(message)),
            ModuleEvent::Privacy(message) => Some(Message::Privacy(message)),
            ModuleEvent::Settings(message) => Some(Message::Settings(message)),
            ModuleEvent::MediaPlayer(message) => Some(Message::MediaPlayer(message)),
            ModuleEvent::Notifications(message) => Some(Message::Notifications(message)),
            ModuleEvent::Custom {
                name,
                message
            } => Some(Message::CustomUpdate(name.as_ref().to_owned(), message)),
            _ => None
        }
    }
}
