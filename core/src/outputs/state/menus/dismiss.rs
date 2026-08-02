//! Dismissal of open menus by a press landing on a bar surface.
//!
//! A press on the bar is the one place the backdrop of a menu cannot cover,
//! so every open menu is armed on the down-stroke and taken down only when
//! the completed press was claimed by no module.

use iced::Task;

use super::super::Outputs;

impl Outputs {
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
}
