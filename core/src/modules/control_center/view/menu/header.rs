//! The top row of the settings menu: battery readout, lock and power
//! buttons.

use iced::{
    Element, Length,
    widget::{Row, Space, button}
};

use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale
    },
    config::ControlCenterModuleConfig,
    modules::control_center::state::{ControlCenter, Message, SubMenu},
    style::settings_button_style
};

impl ControlCenter {
    /// Builds the top row of the menu: the battery readout and the lock and
    /// power buttons.
    pub(super) fn menu_header(
        &self,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        let battery_data = self
            .upower
            .as_ref()
            .and_then(|upower| upower.battery)
            .map(|battery| battery.settings_indicator(icons));
        let right_buttons = Row::new()
            .push_maybe(config.lock_cmd.as_ref().map(|_| {
                button(icon(icons, Icons::Lock))
                    .padding([scale::scaled(8.0), scale::scaled(13.0)])
                    .on_press(Message::Lock)
                    .style(settings_button_style(opacity))
            }))
            .push(
                button(icon(
                    icons,
                    if self.sub_menu == Some(SubMenu::Power) {
                        Icons::Close
                    } else {
                        Icons::Power
                    }
                ))
                .padding([scale::scaled(8.0), scale::scaled(13.0)])
                .on_press(Message::ToggleSubMenu(SubMenu::Power))
                .style(settings_button_style(opacity))
            )
            .spacing(scale::scaled(8.0));

        Row::new()
            .push_maybe(battery_data)
            .push(Space::new().width(Length::Fill))
            .push(right_buttons)
            .spacing(scale::scaled(8.0))
            .width(Length::Fill)
            .into()
    }
}
