//! Lifetime and fade of a single menu surface.

use std::time::Duration;

use iced::{
    KeyboardInteractivity, Layer, SurfaceId as Id, Task, set_keyboard_interactivity, set_layer
};

use super::kind::MenuType;
use crate::{animation::Spring, config::AnimationConfig, position_button::ButtonUIRef};

#[derive(Clone, Debug)]
pub struct Menu {
    pub id:        Id,
    pub menu_info: Option<(MenuType, ButtonUIRef)>,
    opacity:       Spring,
    /// Whether the press in flight has to take this menu down once it
    /// completes.
    dismiss_armed: bool
}

impl Menu {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            menu_info: None,
            opacity: Spring::new(0.0),
            dismiss_armed: false
        }
    }

    /// Arms the open menu for dismissal by the press currently in flight.
    ///
    /// Arming is deliberately silent. The menu is only taken down once that
    /// press completes without a module having claimed it, so a press that
    /// turns out to belong to another module switches the menu over instead of
    /// flashing the surface off and on again.
    pub fn arm_dismissal(&mut self) {
        self.dismiss_armed = self.menu_info.is_some();
    }

    /// Closes the menu when the press that armed it completed elsewhere.
    ///
    /// A menu a module toggled in the meantime is no longer armed, so the press
    /// that opened it does not immediately take it back down.
    pub fn dismiss_if_armed<Message: 'static>(
        &mut self,
        config: &crate::config::Config
    ) -> Task<Message> {
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
        config: &crate::config::Config
    ) -> Task<Message> {
        self.menu_info.replace((menu_type, button_ui_ref));
        self.dismiss_armed = false;

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

    pub fn close<Message: 'static>(&mut self, config: &crate::config::Config) -> Task<Message> {
        self.dismiss_armed = false;

        if self.menu_info.is_some() {
            self.menu_info.take();

            self.aim_opacity(0.0, &config.appearance.animations);

            let mut tasks = vec![set_layer(self.id, Layer::Background)];

            if config.menu_keyboard_focus {
                tasks.push(set_keyboard_interactivity(
                    self.id,
                    KeyboardInteractivity::None
                ));
            }

            Task::batch(tasks)
        } else {
            Task::none()
        }
    }

    pub fn toggle<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &crate::config::Config
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
        menu_type: MenuType,
        config: &crate::config::Config
    ) -> Task<Message> {
        if let Some((current_type, _)) = self.menu_info.as_ref() {
            if *current_type == menu_type {
                self.close(config)
            } else {
                Task::none()
            }
        } else {
            Task::none()
        }
    }

    pub fn request_keyboard<Message: 'static>(&self, menu_keyboard_focus: bool) -> Task<Message> {
        if menu_keyboard_focus {
            set_keyboard_interactivity(self.id, KeyboardInteractivity::OnDemand)
        } else {
            Task::none()
        }
    }

    pub fn release_keyboard<Message: 'static>(&self, menu_keyboard_focus: bool) -> Task<Message> {
        if menu_keyboard_focus {
            set_keyboard_interactivity(self.id, KeyboardInteractivity::None)
        } else {
            Task::none()
        }
    }

    /// Points the opacity spring at `target`, or jumps to it when animations
    /// are disabled.
    fn aim_opacity(&mut self, target: f32, animation_config: &AnimationConfig) {
        if animation_config.enabled {
            self.opacity.set_response(Duration::from_millis(
                animation_config.menu_fade_duration_ms
            ));
            self.opacity.set_target(target);
        } else {
            self.opacity.snap_to(target);
        }
    }

    /// Advances the opacity spring by `elapsed` and reports whether the menu
    /// still needs frames.
    pub fn tick_animation(
        &mut self,
        animation_config: &AnimationConfig,
        elapsed: Duration
    ) -> bool {
        if !animation_config.enabled {
            return false;
        }

        self.opacity.advance(elapsed)
    }

    /// Returns whether the menu has an unfinished opacity animation.
    pub fn is_animating(&self) -> bool {
        self.opacity.is_animating()
    }

    /// Get the current animated opacity for rendering
    pub fn get_opacity(&self) -> f32 {
        self.opacity.value()
    }
}
