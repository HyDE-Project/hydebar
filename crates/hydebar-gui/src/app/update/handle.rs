//! Dispatch of a single application message.

use iced::Task;

use super::super::state::{App, Message};

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::None => Task::none(),
            Message::Demo => self.update_demo(),
            Message::Frame(_)
            | Message::BusFlushed(_)
            | Message::ConfigChanged(_)
            | Message::ConfigDegraded(_)
            | Message::Shutdown(_) => self.update_lifecycle(message),
            Message::ToggleMenu(..)
            | Message::ModuleTooltip(..)
            | Message::CloseMenu(_)
            | Message::CloseAllMenus
            | Message::BarPressed
            | Message::BarReleased => self.update_menus(message),
            Message::ActivateNavigationMode
            | Message::DeactivateNavigationMode
            | Message::NavigateUp
            | Message::NavigateDown
            | Message::NavigateLeft
            | Message::NavigateRight
            | Message::ActivateFocusedModule => self.update_navigation(message),
            Message::OutputEvent(_) => self.update_outputs(message),
            other => self.update_modules(other)
        }
    }
}
