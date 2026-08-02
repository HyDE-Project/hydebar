//! Bar entry and menu of the standalone power profile module.

use iced::{Element, Length, widget::Column};

use super::super::helpers::quick_settings_section;
use crate::{
    components::{icons::IconTheme, scale},
    config::ControlCenterModuleConfig,
    menu::MenuType,
    modules::{
        OnModulePress,
        control_center::{
            power::power_menu,
            state::{ControlCenter, Message}
        }
    }
};

impl ControlCenter {
    /// Bar entry of the standalone power profile module.
    #[must_use]
    pub fn power_profile_bar<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>
    {
        let indicator = self
            .upower
            .as_ref()
            .and_then(|p| p.power_profile.indicator(icons))?;

        Some((
            indicator,
            Some(OnModulePress::ToggleMenu(MenuType::PowerProfile))
        ))
    }

    /// Menu of the standalone power profile module, with the power
    /// actions underneath.
    pub fn power_profile_menu(
        &self,
        opacity: f32,
        config: &ControlCenterModuleConfig,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        let profile = self
            .upower
            .as_ref()
            .and_then(|u| u.power_profile.get_quick_setting_button(opacity, icons));

        Column::new()
            .push(quick_settings_section(
                profile.into_iter().collect::<Vec<_>>(),
                opacity
            ))
            .push(power_menu(opacity, config, icons).map(Message::Power))
            .width(Length::Fill)
            .spacing(scale::scaled(16.0))
            .into()
    }
}
