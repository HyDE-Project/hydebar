//! The per-surface dispatch: what each window of the bar draws.

use hydebar_core::{
    menu::menu_wrapper, notifications_popup, outputs::HasOutput, tooltip::tooltip_wrapper
};
use iced::{Element, SurfaceId as Id, widget::Row};

use super::super::state::{App, Message};

impl App {
    #[must_use]
    /// Draws whatever this surface is: the bar itself, or a menu it opened.
    pub fn view(&self, id: Id) -> Element<'_, Message> {
        match self.outputs.has(id) {
            Some(HasOutput::Main) => self.bar_surface(id),
            Some(HasOutput::Menu(menu_info)) => {
                let menu_opacity = self.config.appearance.menu.opacity;
                let menu_progress = self.outputs.get_menu_progress(id);

                let menu = menu_info.and_then(|(menu_type, button_ui_ref)| {
                    self.menu_page(menu_type, id, menu_opacity).map(
                        |(content, size, measured_height)| {
                            let layout = measured_height.map_or_else(
                                || self.menu_layout(menu_opacity, menu_progress),
                                |height| {
                                    self.measured_menu_layout(menu_opacity, menu_progress, height)
                                }
                            );

                            menu_wrapper(
                                id,
                                content,
                                size,
                                *button_ui_ref,
                                layout,
                                Message::None,
                                Message::CloseMenu(id)
                            )
                        }
                    )
                });

                menu.map_or_else(
                    || self.screen_greeting(),
                    |menu| self.faded_menu(menu, menu_progress)
                )
            }
            Some(HasOutput::Desk) => self.desk_surface(id),
            Some(HasOutput::Notifications) => notifications_popup::view(
                &self.notification_popups,
                self.appearance(),
                self.config.position
            ),
            Some(HasOutput::Tooltip) => self.outputs.tooltip(id).map_or_else(
                || Row::new().into(),
                |info| {
                    self.faded_menu(
                        tooltip_wrapper(info, self.config.position, self.appearance()),
                        self.hints.presence()
                    )
                }
            ),
            None => Row::new().into()
        }
    }
}
