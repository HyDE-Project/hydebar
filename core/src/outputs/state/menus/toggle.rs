//! Opening and closing menus across the tracked outputs.
//!
//! A toggle on one surface closes the menus of every other output — only one
//! menu is ever on screen — and the close variants narrow that same walk by
//! surface, by menu type or not at all.

use iced::{SurfaceId as Id, Task};

use super::super::Outputs;
use crate::{menu::MenuType, position_button::ButtonUIRef};

impl Outputs {
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
        let hide_tooltip = self.hide_tooltip(id, None);

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
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the owned menu type keeps the public signature stable for callers"
    )]
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
            Some((_, Some(shell_info), _)) => shell_info.menu.close_if(&menu_type, config),
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
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the owned menu type keeps the public signature stable for callers"
    )]
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
                        shell_info.menu.close_if(&menu_type, config)
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
                        if shell_info.menu.is_open() {
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
