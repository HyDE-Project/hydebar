//! Lifetime and fade of a single menu surface.

use std::time::Duration;

use iced::{
    KeyboardInteractivity, Layer, OutputId, SurfaceId as Id, Task, destroy_layer_surface,
    new_layer_surface, set_keyboard_interactivity, set_layer
};

use super::kind::MenuType;
use crate::{
    animation::Spring, config::AnimationConfig, outputs::wayland::menu_settings,
    position_button::ButtonUIRef
};

/// Opacity distance below which the fade counts as arrived.
///
/// The default spring precision is tuned for values whose last thousandths
/// still matter. An opacity's do not — yet the spring walked them anyway,
/// and for that whole asymptotic crawl a large dark panel stayed readable
/// over a bright wallpaper as a ghost of the closed window. Settling at the
/// first invisible alpha drops the surface the moment the eye is done with
/// it, without touching the visible part of either fade.
const SETTLE_PRECISION: f32 = 0.02;

#[derive(Clone, Debug)]
pub struct Menu {
    pub id:        Id,
    pub menu_info: Option<(MenuType, ButtonUIRef)>,
    /// Output the menu surface stands on, for building its replacement.
    output:        Option<OutputId>,
    opacity:       Spring,
    /// Opacity the menu opens up to, kept so the travelled share can be told
    /// while fading in.
    full_opacity:  f32,
    /// Whether the press in flight has to take this menu down once it
    /// completes.
    dismiss_armed: bool
}

impl Menu {
    pub fn new(id: Id, output: Option<OutputId>) -> Self {
        Self {
            id,
            menu_info: None,
            output,
            opacity: Spring::new(0.0).with_precision(SETTLE_PRECISION),
            full_opacity: 0.0,
            dismiss_armed: false
        }
    }

    /// Returns whether the menu is open as far as the user is concerned.
    pub fn is_open(&self) -> bool {
        self.menu_info.is_some()
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
    pub fn close<Message: 'static>(&mut self, _config: &crate::config::Config) -> Task<Message> {
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
    /// still needs frames.
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

        (running, Task::none())
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
