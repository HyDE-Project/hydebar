//! Lifetime and fade of a single menu surface.

use std::time::Duration;

use iced::{
    Task,
    platform_specific::shell::wayland::commands::layer_surface::{
        KeyboardInteractivity, Layer, set_keyboard_interactivity, set_layer
    },
    window::Id
};

use super::kind::MenuType;
use crate::{animation::Spring, config::AnimationConfig, position_button::ButtonUIRef};

#[derive(Clone, Debug)]
pub struct Menu {
    pub id:        Id,
    pub menu_info: Option<(MenuType, ButtonUIRef)>,
    opacity:       Spring
}

impl Menu {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            menu_info: None,
            opacity: Spring::new(0.0)
        }
    }

    pub fn open<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &crate::config::Config
    ) -> Task<Message> {
        self.menu_info.replace((menu_type, button_ui_ref));

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
