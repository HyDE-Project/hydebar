//! TEMPORARY verification hook, removed before the change is finished.

use std::{
    env,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration
};

use hydebar_core::{menu::MenuType, modules::settings, position_button::ButtonUIRef};
use iced::{Point, Subscription, Task};

use super::super::state::{App, Message};

const VAR: &str = "HYDEBAR_DEMO_THEME";

static FIRED: AtomicBool = AtomicBool::new(false);

impl App {
    pub(super) fn demo_subscription(&self) -> Subscription<Message> {
        if env::var(VAR).is_ok() {
            iced::time::every(Duration::from_secs(2)).map(|_| Message::Demo)
        } else {
            Subscription::none()
        }
    }

    pub(super) fn update_demo(&mut self) -> Task<Message> {
        let Ok(theme) = env::var(VAR) else {
            return Task::none();
        };

        if !self.outputs.menu_is_open() {
            let Some(id) = self.outputs.first_main_window_id() else {
                return Task::none();
            };

            let opened = self.outputs.toggle_menu(
                id,
                MenuType::Settings,
                ButtonUIRef {
                    position: Point::new(3500.0, 40.0),
                    viewport: (3840.0, 2160.0)
                },
                &self.config
            );
            let tab = self.update(Message::Settings(settings::Message::SelectTab(
                settings::Tab::Hyde
            )));

            return Task::batch([opened, tab]);
        }

        if FIRED.swap(true, Ordering::SeqCst) {
            return Task::none();
        }

        self.update(Message::Settings(settings::Message::SwitchHydeTheme(theme)))
    }
}
