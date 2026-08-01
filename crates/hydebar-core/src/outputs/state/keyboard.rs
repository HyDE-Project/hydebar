//! Keyboard focus requests for menu surfaces.

use iced::{SurfaceId as Id, Task};

use super::Outputs;

impl Outputs {
    /// Request keyboard focus for the menu associated with the identifier.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// outputs.request_keyboard(surface_id, true);
    /// ```
    #[must_use]
    pub fn request_keyboard<Message: 'static>(
        &self,
        id: Id,
        menu_keyboard_focus: bool
    ) -> Task<Message> {
        match self.0.iter().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => {
                shell_info.menu.request_keyboard(menu_keyboard_focus)
            }
            _ => Task::none()
        }
    }

    /// Release keyboard focus from the identified menu surface.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// outputs.release_keyboard(surface_id, false);
    /// ```
    #[must_use]
    pub fn release_keyboard<Message: 'static>(
        &self,
        id: Id,
        menu_keyboard_focus: bool
    ) -> Task<Message> {
        match self.0.iter().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => {
                shell_info.menu.release_keyboard(menu_keyboard_focus)
            }
            _ => Task::none()
        }
    }

    /// Identifiers of every bar surface currently on screen.
    pub fn main_ids(&self) -> impl Iterator<Item = Id> + '_ {
        self.0
            .iter()
            .filter_map(|(_, shell_info, _)| shell_info.as_ref().map(|shell| shell.id))
    }

    /// Identifiers of every notification surface currently on screen.
    pub fn notification_ids(&self) -> impl Iterator<Item = Id> + '_ {
        self.0.iter().filter_map(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell| shell.notifications_id)
        })
    }

    /// Returns the first main window Id if any outputs exist.
    #[must_use]
    pub fn first_main_window_id(&self) -> Option<Id> {
        self.0
            .iter()
            .find_map(|(_, shell_info, _)| shell_info.as_ref().map(|s| s.id))
    }
}
