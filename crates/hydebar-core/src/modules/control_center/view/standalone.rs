//! Bar entries and menus of the modules split out of the control
//! center.
//!
//! Audio, network, bluetooth and the power profile each render their
//! own bar entry and own menu, while the services behind them
//! stay in the single [`ControlCenter`] state: splitting the
//! presentation must not multiply the D-Bus connections the bar
//! keeps open.

use iced::Element;

use super::quick_setting_button;
use crate::{
    components::icons::{IconTheme, Icons},
    modules::control_center::state::{ControlCenter, Message}
};

mod audio;
mod bluetooth;
mod brightness;
mod network;
mod power_profile;

impl ControlCenter {
    /// Quick toggle of the idle inhibitor, shared by the control center
    /// menu.
    #[allow(dead_code)]
    pub(super) fn idle_quick_button(
        &self,
        opacity: f32,
        icons: &IconTheme
    ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
        self.idle_inhibitor.as_ref().map(|inhibitor| {
            (
                quick_setting_button(
                    icons,
                    if inhibitor.is_inhibited() {
                        Icons::EyeOpened
                    } else {
                        Icons::EyeClosed
                    },
                    "Idle Inhibitor".to_string(),
                    None,
                    inhibitor.is_inhibited(),
                    Message::ToggleInhibitIdle,
                    None,
                    opacity
                ),
                None
            )
        })
    }
}
