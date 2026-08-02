//! Bar entry and menu of the standalone network module.

use iced::{Element, SurfaceId as Id, widget::Row};

use super::super::helpers::quick_settings_section;
use crate::{
    components::{icons::IconTheme, push_maybe::PushMaybe, scale},
    config::ControlCenterModuleConfig,
    menu::MenuType,
    modules::{
        OnModulePress,
        control_center::state::{ControlCenter, Message}
    },
    password_dialog
};

impl ControlCenter {
    /// Bar entry of the standalone network module, connection and VPN.
    #[must_use]
    pub fn network_bar<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>
    {
        let network = self.network.as_ref()?;
        let connection = network.get_connection_indicator(icons);
        let vpn = network.get_vpn_indicator(icons);

        if connection.is_none() && vpn.is_none() {
            return None;
        }

        Some((
            Row::new()
                .push_maybe(connection)
                .push_maybe(vpn)
                .spacing(scale::icon_gap())
                .into(),
            Some(OnModulePress::ToggleMenu(MenuType::Network))
        ))
    }

    /// Menu of the standalone network module: connection, VPN and
    /// airplane mode.
    ///
    /// The password prompt of a protected network belongs here as well,
    /// since this is the menu the connection attempt starts
    /// from.
    pub fn network_menu(
        &self,
        id: Id,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        if let Some((ssid, current_password)) = &self.password_dialog {
            return password_dialog::view(id, ssid, current_password, opacity, icons)
                .map(Message::PasswordDialog);
        }

        let buttons = vec![
            self.network.as_ref().and_then(|n| {
                n.get_wifi_quick_setting_button(
                    id,
                    self.sub_menu,
                    config.wifi_more_cmd.is_some(),
                    opacity,
                    icons
                )
            }),
            self.network.as_ref().and_then(|n| {
                n.get_vpn_quick_setting_button(
                    id,
                    self.sub_menu,
                    config.vpn_more_cmd.is_some(),
                    opacity,
                    icons
                )
            }),
            self.network.as_ref().and_then(|n| {
                if config.remove_airplane_btn {
                    None
                } else {
                    Some(n.get_airplane_mode_quick_setting_button(opacity, icons))
                }
            }),
        ];

        quick_settings_section(buttons.into_iter().flatten().collect::<Vec<_>>(), opacity)
    }
}
