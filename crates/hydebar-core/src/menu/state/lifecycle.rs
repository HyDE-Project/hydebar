//! Opening, closing and press-driven dismissal of the menu surface.

use iced::{
    KeyboardInteractivity, Layer, Task, destroy_layer_surface, new_layer_surface,
    set_keyboard_interactivity, set_layer
};

use super::Menu;
use crate::{
    config::Config, menu::kind::MenuType, outputs::wayland::menu_settings,
    position_button::ButtonUIRef
};

impl Menu {
    /// Arms the open menu for dismissal by the press currently in flight.
    ///
    /// Arming is deliberately silent. The menu is only taken down once that
    /// press completes without a module having claimed it, so a press that
    /// turns out to belong to another module switches the menu over instead of
    /// flashing the surface off and on again.
    pub const fn arm_dismissal(&mut self) {
        self.dismiss_armed = self.is_open();
    }

    /// Closes the menu when the press that armed it completed elsewhere.
    ///
    /// A menu a module toggled in the meantime is no longer armed, so the press
    /// that opened it does not immediately take it back down.
    pub fn dismiss_if_armed<Message: 'static>(&mut self, config: &Config) -> Task<Message> {
        if std::mem::take(&mut self.dismiss_armed) {
            self.close(config)
        } else {
            Task::none()
        }
    }

    pub fn open<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &Config
    ) -> Task<Message> {
        self.menu_info.replace((menu_type, button_ui_ref));
        self.dismiss_armed = false;

        self.full_opacity = config.appearance.menu.opacity;

        self.aim_opacity(
            config.appearance.menu.opacity,
            &config.appearance.animations
        );

        let mut tasks = vec![set_layer(self.id, Layer::Overlay)];

        if config.menu_keyboard_focus {
            tasks.push(set_keyboard_interactivity(
                self.id,
                KeyboardInteractivity::OnDemand
            ));
        }

        Task::batch(tasks)
    }

    /// Takes the menu down by destroying its surface whole.
    ///
    /// The window is not faded element by element: the compositor is handed
    /// the surface's last composited frame and plays its own layer animation
    /// over it, so the box, the content and the backdrop leave the screen as
    /// one layer — the way every other popup on the desktop leaves. A fresh
    /// empty surface is raised in the same breath to host the next open.
    pub fn close<Message: 'static>(&mut self, _config: &Config) -> Task<Message> {
        self.dismiss_armed = false;

        if !self.is_open() {
            return Task::none();
        }

        self.menu_info.take();
        self.opacity.snap_to(0.0);
        self.full_opacity = 0.0;

        let departed = self.id;
        let (successor, raise) = new_layer_surface(menu_settings(self.output));
        self.id = successor;

        Task::batch(vec![destroy_layer_surface(departed), raise])
    }

    pub fn toggle<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &Config
    ) -> Task<Message> {
        self.dismiss_armed = false;

        match self.menu_info.as_mut() {
            None => self.open(menu_type, button_ui_ref, config),
            Some((current_type, _)) if *current_type == menu_type => self.close(config),
            Some((current_type, current_button_ui_ref)) => {
                *current_type = menu_type;
                *current_button_ui_ref = button_ui_ref;
                Task::none()
            }
        }
    }

    pub fn close_if<Message: 'static>(
        &mut self,
        menu_type: &MenuType,
        config: &Config
    ) -> Task<Message> {
        if let Some((current_type, _)) = self.menu_info.as_ref() {
            if current_type == menu_type {
                self.close(config)
            } else {
                Task::none()
            }
        } else {
            Task::none()
        }
    }

    #[must_use]
    pub fn request_keyboard<Message: 'static>(&self, menu_keyboard_focus: bool) -> Task<Message> {
        if menu_keyboard_focus {
            set_keyboard_interactivity(self.id, KeyboardInteractivity::OnDemand)
        } else {
            Task::none()
        }
    }

    #[must_use]
    pub fn release_keyboard<Message: 'static>(&self, menu_keyboard_focus: bool) -> Task<Message> {
        if menu_keyboard_focus {
            set_keyboard_interactivity(self.id, KeyboardInteractivity::None)
        } else {
            Task::none()
        }
    }
}
