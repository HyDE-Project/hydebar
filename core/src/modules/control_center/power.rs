use iced::{
    Element, Length,
    widget::{button, column, row, rule}
};

use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    },
    config::ControlCenterModuleConfig,
    style::ghost_button_style,
    utils
};

/// What the power section of the quick settings answers to.
#[derive(Debug, Clone)]
pub enum PowerMessage {
    /// Suspend the machine, with this command.
    Suspend(String),
    /// Restart the machine, with this command.
    Reboot(String),
    /// Power the machine off, with this command.
    Shutdown(String),
    /// End the session, with this command.
    Logout(String)
}

impl PowerMessage {
    /// Runs the command this asks for.
    pub fn update(self) {
        match self {
            Self::Suspend(cmd) => {
                utils::launcher::suspend(cmd);
            }
            Self::Reboot(cmd) => {
                utils::launcher::reboot(cmd);
            }
            Self::Shutdown(cmd) => {
                utils::launcher::shutdown(cmd);
            }
            Self::Logout(cmd) => {
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
        button(row!(icon(icons, Icons::Suspend), text("Suspend")).spacing(scale::scaled(16.0)))
            .padding([scale::scaled(4.0), scale::scaled(12.0)])
            .on_press(PowerMessage::Suspend(config.suspend_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
        button(row!(icon(icons, Icons::Reboot), text("Reboot")).spacing(scale::scaled(16.0)))
            .padding([scale::scaled(4.0), scale::scaled(12.0)])
            .on_press(PowerMessage::Reboot(config.reboot_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
        button(row!(icon(icons, Icons::Power), text("Shutdown")).spacing(scale::scaled(16.0)))
            .padding([scale::scaled(4.0), scale::scaled(12.0)])
            .on_press(PowerMessage::Shutdown(config.shutdown_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
        rule::horizontal(1),
        button(row!(icon(icons, Icons::Logout), text("Logout")).spacing(scale::scaled(16.0)))
            .padding([scale::scaled(4.0), scale::scaled(12.0)])
            .on_press(PowerMessage::Logout(config.logout_cmd.clone()))
            .width(Length::Fill)
            .style(ghost_button_style(opacity)),
    )
    .padding(scale::scaled(8.0))
    .width(Length::Fill)
    .spacing(scale::scaled(8.0))
    .into()
}
