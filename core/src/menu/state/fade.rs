//! The opacity spring of the menu, doubling as its travel signal.

use std::time::Duration;

use iced::Task;

use super::Menu;
use crate::config::AnimationConfig;

impl Menu {
    /// How far the menu has travelled between closed and open, either way.
    ///
    /// The opacity spring doubles as the progress signal: the travelled share
    /// is the value against the opacity the menu opens up to. The wrapper
    /// slides the window by the remainder, which is what makes a menu grow out
    /// of its module on the way in — and settle back into it on the way out,
    /// since a fading spring walks this share back down.
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.full_opacity <= f32::EPSILON {
            1.0
        } else {
            (self.opacity.value() / self.full_opacity).clamp(0.0, 1.0)
        }
    }

    /// Points the opacity spring at `target`, or jumps to it when animations
    /// are disabled.
    pub(super) const fn aim_opacity(&mut self, target: f32, animation_config: &AnimationConfig) {
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
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.opacity.is_animating()
    }

    /// Get the current animated opacity for rendering
    #[must_use]
    pub const fn get_opacity(&self) -> f32 {
        self.opacity.value()
    }
}
