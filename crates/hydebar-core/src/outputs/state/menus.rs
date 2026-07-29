//! Menu visibility and fade animation state.

use iced::{Task, window::Id};

use super::Outputs;
use crate::{menu::MenuType, position_button::ButtonUIRef};

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
    pub fn menu_is_open(&self) -> bool {
        self.0.iter().any(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .map(|shell_info| shell_info.menu.menu_info.is_some())
                .unwrap_or_default()
        })
    }

    /// Get the animated opacity for a menu window.
    pub fn get_menu_opacity(&self, id: Id) -> f32 {
        self.0
            .iter()
            .find_map(|(_, shell_info, _)| {
                shell_info.as_ref().and_then(|shell_info| {
                    if shell_info.menu.id == id {
                        Some(shell_info.menu.get_opacity())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(0.0)
    }

    /// Update menu animations. Returns true if any menu is currently animating.
    pub fn tick_menu_animations(
        &mut self,
        animation_config: &crate::config::AnimationConfig,
        elapsed: std::time::Duration
    ) -> bool {
        let mut is_animating = false;
        for (_, shell_info, _) in &mut self.0 {
            if let Some(shell_info) = shell_info
                && shell_info.menu.tick_animation(animation_config, elapsed)
            {
                is_animating = true;
            }
        }
        is_animating
    }

    /// Returns whether any menu still has an unfinished animation.
    pub fn menu_is_animating(&self) -> bool {
        self.0.iter().any(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .map(|shell_info| shell_info.menu.is_animating())
                .unwrap_or_default()
        })
    }

    /// Arm every open menu for dismissal by the press currently in flight.
    ///
    /// Called when a press lands on a bar surface, which is the one place the
    /// backdrop of a menu cannot cover. Nothing is taken down yet: the module
    /// the press landed on is given the whole click to claim it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (mut outputs, _task) =
    ///     Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// outputs.arm_menu_dismissal();
    /// ```
    pub fn arm_menu_dismissal(&mut self) {
        for (_, shell_info, _) in &mut self.0 {
            if let Some(shell_info) = shell_info {
                shell_info.menu.arm_dismissal();
            }
        }
    }

    /// Close every menu the completed press armed and no module claimed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (mut outputs, _task) =
    ///     Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// outputs.dismiss_armed_menus::<()>(&config);
    /// ```
    pub fn dismiss_armed_menus<Message: 'static>(
        &mut self,
        config: &crate::config::Config
    ) -> Task<Message> {
        Task::batch(
            self.0
                .iter_mut()
                .map(|(_, shell_info, _)| match shell_info {
                    Some(shell_info) => shell_info.menu.dismiss_if_armed(config),
                    None => Task::none()
                })
                .collect::<Vec<_>>()
        )
    }

    /// Toggle the menu associated with the provided surface identifier.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let task = outputs.toggle_menu(surface_id, MenuType::Tray("battery".into()), button_ref, &config);
    /// spawn(task);
    /// ```
    pub fn toggle_menu<Message: 'static>(
        &mut self,
        id: Id,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &crate::config::Config
    ) -> Task<Message> {
        let hide_tooltip = self.hide_tooltip(id);

        match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => {
                let toggle_task = shell_info.menu.toggle(menu_type, button_ui_ref, config);
                let mut tasks = self
                    .0
                    .iter_mut()
                    .filter_map(|(_, shell_info, _)| {
                        if let Some(shell_info) = shell_info {
                            if shell_info.id != id && shell_info.menu.id != id {
                                Some(shell_info.menu.close(config))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                tasks.push(toggle_task);
                tasks.push(hide_tooltip);
                Task::batch(tasks)
            }
            _ => hide_tooltip
        }
    }

    /// Close the menu for a specific surface when it is currently open.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// outputs.close_menu(surface_id, &config);
    /// ```
    pub fn close_menu<Message: 'static>(
        &mut self,
        id: Id,
        config: &crate::config::Config
    ) -> Task<Message> {
        match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.close(config),
            _ => Task::none()
        }
    }

    /// Close the menu only when it matches the specified [`MenuType`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// outputs.close_menu_if(surface_id, MenuType::Updates, &config);
    /// ```
    pub fn close_menu_if<Message: 'static>(
        &mut self,
        id: Id,
        menu_type: MenuType,
        config: &crate::config::Config
    ) -> Task<Message> {
        match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.close_if(menu_type, config),
            _ => Task::none()
        }
    }

    /// Close every menu that matches the specified [`MenuType`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// outputs.close_all_menu_if(MenuType::Tray("network".into()), &config);
    /// ```
    pub fn close_all_menu_if<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        config: &crate::config::Config
    ) -> Task<Message> {
        Task::batch(
            self.0
                .iter_mut()
                .map(|(_, shell_info, _)| {
                    if let Some(shell_info) = shell_info {
                        shell_info.menu.close_if(menu_type.clone(), config)
                    } else {
                        Task::none()
                    }
                })
                .collect::<Vec<_>>()
        )
    }

    /// Close every open menu regardless of its type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (mut outputs, _task) =
    ///     Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// outputs.close_all_menus::<()>(&config);
    /// ```
    pub fn close_all_menus<Message: 'static>(
        &mut self,
        config: &crate::config::Config
    ) -> Task<Message> {
        Task::batch(
            self.0
                .iter_mut()
                .map(|(_, shell_info, _)| {
                    if let Some(shell_info) = shell_info {
                        if shell_info.menu.menu_info.is_some() {
                            shell_info.menu.close(config)
                        } else {
                            Task::none()
                        }
                    } else {
                        Task::none()
                    }
                })
                .collect::<Vec<_>>()
        )
    }
}
