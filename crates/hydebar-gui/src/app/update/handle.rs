//! Dispatch of a single application message.

use std::sync::atomic::{AtomicBool, Ordering};

use hydebar_core::{menu::MenuType, position_button::ButtonUIRef};
use iced::Task;

use super::super::state::{App, Message};

static SHOT: AtomicBool = AtomicBool::new(false);

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        if std::env::var_os("HYDEBAR_SHOT").is_some() && !SHOT.load(Ordering::Relaxed) {
            if let Some(id) = self.outputs.first_main_window_id() {
                SHOT.store(true, Ordering::Relaxed);

                return self.update(Message::ToggleMenu(
                    MenuType::Themes,
                    id,
                    ButtonUIRef {
                        position: iced::Point {
                            x: 960.0, y: 20.0
                        },
                        viewport: (1920.0, 1080.0)
                    }
                ));
            }
        }

        match message {
            Message::None => Task::none(),
            Message::Frame(_)
            | Message::BusFlushed(_)
            | Message::ConfigChanged(_)
            | Message::ConfigDegraded(_)
            | Message::Shutdown(_)
            | Message::PollAtRest
            | Message::PollAttended => self.update_lifecycle(message),
            Message::ToggleMenu(..)
            | Message::ModuleHover {
                ..
            }
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
