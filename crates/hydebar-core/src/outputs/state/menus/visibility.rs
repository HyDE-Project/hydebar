//! What the tracked menus currently show and how far their fades travelled.
//!
//! Read-only answers about the menu surfaces — which one is open, how far an
//! animation has moved — plus the per-frame tick that advances every fade at
//! once.

use iced::{SurfaceId as Id, Task};

use super::super::Outputs;
use crate::menu::MenuType;

impl Outputs {
    /// Determine whether any tracked menu surface is currently visible.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// assert!(!outputs.menu_is_open());
    /// ```
    #[must_use]
    pub fn menu_is_open(&self) -> bool {
        self.0.iter().any(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .is_some_and(|shell_info| shell_info.menu.is_open())
        })
    }

    /// The menu currently on screen, if any.
    ///
    /// Only one menu is ever open at a time, so the first one found is the one
    /// the user is looking at.
    #[must_use]
    pub fn open_menu(&self) -> Option<&MenuType> {
        self.0.iter().find_map(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .filter(|shell_info| shell_info.menu.is_open())
                .and_then(|shell_info| shell_info.menu.menu_info.as_ref())
                .map(|(menu_type, _)| menu_type)
        })
    }

    /// Get the animated opacity for a menu window.
    /// How far the open animation of the menu on `id` has travelled.
    #[must_use]
    pub fn get_menu_progress(&self, id: Id) -> f32 {
        self.0
            .iter()
            .find_map(|(_, shell_info, _)| {
                shell_info.as_ref().and_then(|shell_info| {
                    if shell_info.menu.id == id {
                        Some(shell_info.menu.progress())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(1.0)
    }

    /// Menu surface of every output, with whether a menu is open on it.
    ///
    /// The greeting borrows these surfaces while the bar is born: they span
    /// the screen and are idle at that moment, and the caller must know which
    /// ones a menu actually owns before sending any of them back down.
    #[must_use]
    pub fn menu_surfaces(&self) -> Vec<(Id, bool)> {
        self.0
            .iter()
            .filter_map(|(_, shell_info, _)| {
                shell_info
                    .as_ref()
                    .map(|shell_info| (shell_info.menu.id, shell_info.menu.is_open()))
            })
            .collect()
    }

    /// Update menu animations. Returns whether any menu is still animating,
    /// together with the tasks finishing the closes that just completed.
    pub fn tick_menu_animations<Message: 'static>(
        &mut self,
        animation_config: &crate::config::AnimationConfig,
        elapsed: std::time::Duration
    ) -> (bool, Task<Message>) {
        let mut is_animating = false;
        let mut tasks = Vec::new();

        for (_, shell_info, _) in &mut self.0 {
            if let Some(shell_info) = shell_info {
                let (running, task) = shell_info.menu.tick_animation(animation_config, elapsed);

                is_animating |= running;
                tasks.push(task);
            }
        }

        (is_animating, Task::batch(tasks))
    }

    /// Returns whether any menu still has an unfinished animation.
    #[must_use]
    pub fn menu_is_animating(&self) -> bool {
        self.0.iter().any(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .is_some_and(|shell_info| shell_info.menu.is_animating())
        })
    }
}
