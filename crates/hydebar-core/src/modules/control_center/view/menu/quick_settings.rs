//! The grid of quick setting toggles inside the settings menu.

use iced::{Element, SurfaceId as Id};

use super::super::{helpers::quick_settings_section, quick_setting_button};
use crate::{
    components::icons::{IconTheme, Icons},
    config::ControlCenterModuleConfig,
    modules::control_center::state::{ControlCenter, Message},
    services::bluetooth::BluetoothState
};

impl ControlCenter {
    /// Builds the grid of quick setting toggles and their unfolded menus.
    pub(super) fn menu_quick_settings(
        &self,
        id: Id,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        let wifi_setting_button = self.network.as_ref().and_then(|n| {
            n.get_wifi_quick_setting_button(
                id,
                self.sub_menu,
                config.wifi_more_cmd.is_some(),
                opacity,
                icons
            )
        });

        quick_settings_section(
            vec![
                wifi_setting_button,
                self.bluetooth
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
                self.idle_inhibitor.as_ref().and_then(|i| {
                    if config.remove_idle_btn {
                        None
                    } else {
                        Some((
                            quick_setting_button(
                                icons,
                                if i.is_inhibited() {
                                    Icons::EyeOpened
                                } else {
                                    Icons::EyeClosed
                                },
                                "Idle Inhibitor".to_string(),
                                None,
                                i.is_inhibited(),
                                Message::ToggleInhibitIdle,
                                None,
                                opacity
                            ),
                            None
                        ))
                    }
                }),
                self.upower
                    .as_ref()
                    .and_then(|u| u.power_profile.get_quick_setting_button(opacity, icons)),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
            opacity
        )
    }
}
