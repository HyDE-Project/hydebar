use iced::{
    Element, Length,
    widget::{button, column, row, rule, text}
};

use crate::{
    components::icons::{IconTheme, Icons, icon},
    config::ControlCenterModuleConfig,
    style::ghost_button_style,
    utils
};

#[derive(Debug, Clone)]
pub enum PowerMessage {
    Suspend(String),
    Reboot(String),
    Shutdown(String),
    Logout(String)
}

impl PowerMessage {
    pub fn update(self) {
        match self {
            PowerMessage::Suspend(cmd) => {
                utils::launcher::suspend(cmd);
            }
            PowerMessage::Reboot(cmd) => {
                utils::launcher::reboot(cmd);
            }
            PowerMessage::Shutdown(cmd) => {
                utils::launcher::shutdown(cmd);
            }
            PowerMessage::Logout(cmd) => {
                utils::launcher::logout(cmd);
            }
        }
    }
}

pub fn power_menu<'a>(
    opacity: f32,
    config: &ControlCenterModuleConfig,
    icons: &IconTheme
) -> Element<'a, PowerMessage> {
    column!(
        button(row!(icon(icons, Icons::Suspend), text("Suspend")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Suspend(config.suspend_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
        button(row!(icon(icons, Icons::Reboot), text("Reboot")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Reboot(config.reboot_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
        button(row!(icon(icons, Icons::Power), text("Shutdown")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Shutdown(config.shutdown_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
        rule::horizontal(1),
        button(row!(icon(icons, Icons::Logout), text("Logout")).spacing(16))
            .padding([4, 12])
            .on_press(PowerMessage::Logout(config.logout_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
    )
    .padding(8)
    .width(Length::Fill)
    .spacing(8)
    .into()
}
