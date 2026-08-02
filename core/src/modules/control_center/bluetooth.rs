use iced::{
    Element, Length, SurfaceId as Id, Theme,
    widget::{Column, Row, button, column, container, row, rule}
};

use super::{Message, SubMenu, quick_setting_button};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale,
        text::text
    },
    services::{
        ServiceEvent,
        bluetooth::{BluetoothData, BluetoothService, BluetoothState}
    },
    style::ghost_button_style
};

#[derive(Debug, Clone)]
pub enum BluetoothMessage {
    Event(Box<ServiceEvent<BluetoothService>>),
    Toggle,
    ConnectDevice(zbus::zvariant::OwnedObjectPath),
    DisconnectDevice(zbus::zvariant::OwnedObjectPath),
    More(Id)
}

impl BluetoothData {
    #[must_use]
    pub fn get_quick_setting_button(
        &self,
        id: Id,
        sub_menu: Option<SubMenu>,
        show_more_button: bool,
        opacity: f32,
        icons: &IconTheme
    ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
        Some((
            quick_setting_button(
                icons,
                Icons::Bluetooth,
                "Bluetooth".to_owned(),
                None,
                self.state == BluetoothState::Active,
                Message::Bluetooth(BluetoothMessage::Toggle),
                (self.state == BluetoothState::Active).then(|| {
                    (
                        SubMenu::Bluetooth,
                        sub_menu,
                        Message::ToggleSubMenu(SubMenu::Bluetooth)
                    )
                }),
                opacity
            ),
            sub_menu
                .filter(|menu_type| *menu_type == SubMenu::Bluetooth)
                .map(|_| self.bluetooth_menu(id, show_more_button, opacity, icons))
        ))
    }

    #[must_use]
    pub fn bluetooth_menu(
        &self,
        id: Id,
        show_more_button: bool,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        let main = if self.devices.is_empty() {
            container(text("No paired devices"))
                .width(Length::Fill)
                .into()
        } else {
            Column::with_children(self.devices.iter().map(|d| {
                Row::new()
                    .push(text(d.name.clone()).width(Length::Fill))
                    .push_maybe(d.battery.map(|battery| Self::battery_level(battery, icons)))
                    .push(
                        iced::widget::mouse_area(
                            button(text(if d.connected { "Disconnect" } else { "Connect" }))
                                .padding([scale::scaled(4.0), scale::scaled(12.0)])
                                .style(ghost_button_style(opacity))
                        )
                        .on_press(Message::Bluetooth(if d.connected {
                            BluetoothMessage::DisconnectDevice(d.path.clone())
                        } else {
                            BluetoothMessage::ConnectDevice(d.path.clone())
                        }))
                    )
                    .spacing(scale::scaled(8.0))
                    .align_y(iced::Alignment::Center)
                    .into()
            }))
            .spacing(scale::scaled(8.0))
            .width(Length::Fill)
            .into()
        };

        if show_more_button {
            column!(
                main,
                rule::horizontal(1),
                button("More")
                    .on_press(Message::Bluetooth(BluetoothMessage::More(id)))
                    .padding([scale::scaled(4.0), scale::scaled(12.0)])
                    .width(Length::Fill)
                    .style(ghost_button_style(opacity))
            )
            .spacing(scale::scaled(12.0))
            .into()
        } else {
            main
        }
    }

    fn battery_level<'a>(battery: u8, icons: &IconTheme) -> Element<'a, Message> {
        container(
            row!(
                icon(
                    icons,
                    match battery {
                        0..=20 => Icons::Battery0,
                        21..=40 => Icons::Battery1,
                        41..=60 => Icons::Battery2,
                        61..=80 => Icons::Battery3,
                        _ => Icons::Battery4
                    }
                ),
                text(format!("{battery}%"))
            )
            .spacing(scale::scaled(8.0))
            .width(Length::Shrink)
        )
        .style(move |theme: &Theme| container::Style {
            text_color: Some(if battery <= 20 {
                theme.palette().danger
            } else {
                theme.palette().text
            }),
            ..container::Style::default()
        })
        .into()
    }
}
