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
    /// Opacity the menu opens up to, kept so the travelled share can be told
    /// while fading either way.
    full_opacity:  f32,
    /// Whether the menu is playing its way out.
    ///
    /// A closing menu keeps its content and its overlay layer so the fade is
    /// actually visible; only once the spring settles at zero is the surface
    /// emptied and dropped behind everything. For every question except
    /// drawing, a closing menu already counts as closed.
    closing:       bool,
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
            full_opacity: 0.0,
            closing: false,
            dismiss_armed: false
        }
    }

    /// Returns whether the menu is open as far as the user is concerned.
    ///
    /// A menu playing its way out still draws, but it is already closed: it
    /// holds no attention, blocks no toggle and needs no dismissal.
    pub fn is_open(&self) -> bool {
        self.menu_info.is_some() && !self.closing
    }

    /// Arms the open menu for dismissal by the press currently in flight.
    ///
    /// Arming is deliberately silent. The menu is only taken down once that
    /// press completes without a module having claimed it, so a press that
    /// turns out to belong to another module switches the menu over instead of
    /// flashing the surface off and on again.
    pub fn arm_dismissal(&mut self) {
        self.dismiss_armed = self.is_open();
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
        self.closing = false;
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

    /// Starts playing the menu out, or takes it straight down when animations
    /// are off.
    ///
    /// The content and the overlay layer stay while the fade travels — the
    /// window has to be seen settling back into the bar — and the surface is
    /// only emptied and dropped behind everything once the spring reaches
    /// zero, in [`Menu::tick_animation`]. The keyboard is released right away:
    /// a departing window must not keep eating keys.
    pub fn close<Message: 'static>(&mut self, config: &crate::config::Config) -> Task<Message> {
        self.dismiss_armed = false;

        if !self.is_open() {
            return Task::none();
        }

        self.aim_opacity(0.0, &config.appearance.animations);

        let mut tasks = Vec::new();

        if config.menu_keyboard_focus {
            tasks.push(set_keyboard_interactivity(
                self.id,
                KeyboardInteractivity::None
            ));
        }

        if config.appearance.animations.enabled {
            self.closing = true;
        } else {
            self.menu_info.take();
            tasks.push(set_layer(self.id, Layer::Background));
        }

        Task::batch(tasks)
    }

    pub fn toggle<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &crate::config::Config
    ) -> Task<Message> {
        self.dismiss_armed = false;

        if self.closing {
            return self.open(menu_type, button_ui_ref, config);
        }

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

    /// How far the menu has travelled between closed and open, either way.
    ///
    /// The opacity spring doubles as the progress signal: the travelled share
    /// is the value against the opacity the menu opens up to. The wrapper
    /// slides the window by the remainder, which is what makes a menu grow out
    /// of its module on the way in — and settle back into it on the way out,
    /// since a fading spring walks this share back down.
    pub fn progress(&self) -> f32 {
        if self.full_opacity <= f32::EPSILON {
            1.0
        } else {
            (self.opacity.value() / self.full_opacity).clamp(0.0, 1.0)
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
    /// still needs frames, together with the task finishing a completed close.
    ///
    /// A closing menu whose fade just settled is emptied here and its surface
    /// dropped behind everything — the deferred half of [`Menu::close`].
    pub fn tick_animation<Message: 'static>(
        &mut self,
        animation_config: &AnimationConfig,
        elapsed: Duration
    ) -> (bool, Task<Message>) {
        let running = if animation_config.enabled {
            self.opacity.advance(elapsed)
        } else {
            self.opacity.snap_to(self.opacity.target());
            false
        };

        if self.closing && !running {
            self.closing = false;
            self.menu_info.take();

            return (false, set_layer(self.id, Layer::Background));
        }

        (running, Task::none())
    }

    /// Returns whether the menu has an unfinished opacity animation.
    pub fn is_animating(&self) -> bool {
        self.opacity.is_animating()
    }

    /// Returns whether the menu is playing its way out.
    pub fn is_closing(&self) -> bool {
        self.closing
    }

    /// Get the current animated opacity for rendering
    pub fn get_opacity(&self) -> f32 {
        self.opacity.value()
    }
}
