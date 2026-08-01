//! Bar entry and menu of the standalone bluetooth module.

use iced::{Element, SurfaceId as Id};

use super::super::helpers::quick_settings_section;
use crate::{
    components::icons::{IconTheme, Icons, icon},
    config::ControlCenterModuleConfig,
    menu::MenuType,
    modules::{
        OnModulePress,
        control_center::state::{ControlCenter, Message}
    },
    services::bluetooth::BluetoothState
};

impl ControlCenter {
    /// Bar entry of the standalone bluetooth module.
    ///
    /// A machine without a bluetooth radio reports the state as
    /// unavailable and the module stays off the bar.
    #[must_use]
    pub fn bluetooth_bar<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>
    {
        self.bluetooth
            .as_ref()
            .filter(|b| b.state != BluetoothState::Unavailable)?;

        Some((
            icon(icons, Icons::Bluetooth).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Bluetooth))
        ))
    }

    /// Menu of the standalone bluetooth module.
    #[must_use]
    pub fn bluetooth_menu(
        &self,
        id: Id,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        let button = self
            .bluetooth
            .as_ref()
            .filter(|b| b.state != BluetoothState::Unavailable)
            .and_then(|b| {
                b.get_quick_setting_button(
                    id,
                    self.sub_menu,
                    config.bluetooth_more_cmd.is_some(),
                    opacity,
                    icons
                )
            });

        quick_settings_section(button.into_iter().collect::<Vec<_>>(), opacity)
    }
}
