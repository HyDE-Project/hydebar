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

/// What the bluetooth section of the quick settings answers to.
#[derive(Debug, Clone)]
pub enum BluetoothMessage {
    /// The daemon said something.
    Event(Box<ServiceEvent<BluetoothService>>),
    /// Turn the adapter on or off.
    Toggle,
    /// Connect to this device.
    ConnectDevice(zbus::zvariant::OwnedObjectPath),
    /// Disconnect from this device.
    DisconnectDevice(zbus::zvariant::OwnedObjectPath),
    /// Open the full bluetooth settings.
    More(Id)
}

impl BluetoothData {
    /// The bluetooth toggle of the quick settings, where an adapter exists.
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

    /// The list of devices the adapter knows.
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::convert::TryFrom;

    use iced_test::simulator;
    use zbus::zvariant::OwnedObjectPath;

    use super::*;
    use crate::services::bluetooth::BluetoothDevice;

    fn surface() -> Id {
        Id::unique()
    }

    fn icons() -> IconTheme {
        IconTheme::default()
    }

    fn path(at: &str) -> OwnedObjectPath {
        OwnedObjectPath::try_from(at).expect("a well formed object path")
    }

    fn device(name: &str, connected: bool, battery: Option<u8>) -> BluetoothDevice {
        BluetoothDevice {
            name: name.to_owned(),
            battery,
            path: path(&format!("/device/{name}")),
            connected
        }
    }

    fn data(state: BluetoothState, devices: Vec<BluetoothDevice>) -> BluetoothData {
        BluetoothData {
            state,
            devices
        }
    }

    #[test]
    fn the_adapter_is_always_offered_as_a_quick_setting() {
        let data = data(BluetoothState::Inactive, vec![]);

        let (button, submenu) = data
            .get_quick_setting_button(surface(), None, false, 1.0, &icons())
            .expect("the toggle is always there");

        assert!(submenu.is_none());

        let mut ui = simulator(button);
        assert!(ui.find("Bluetooth").is_ok());
    }

    #[test]
    fn pressing_the_toggle_asks_to_switch_the_adapter() {
        let data = data(BluetoothState::Inactive, vec![]);

        let (button, _) = data
            .get_quick_setting_button(surface(), None, false, 1.0, &icons())
            .expect("the toggle is always there");

        let mut ui = simulator(button);
        let _ = ui.click("Bluetooth").expect("the toggle is pressable");

        assert!(
            ui.into_messages()
                .any(|message| matches!(message, Message::Bluetooth(BluetoothMessage::Toggle)))
        );
    }

    #[test]
    fn a_switched_off_adapter_opens_no_device_list() {
        for state in [BluetoothState::Inactive, BluetoothState::Unavailable] {
            let data = data(state, vec![device("buds", false, None)]);

            let (_, submenu) = data
                .get_quick_setting_button(
                    surface(),
                    Some(SubMenu::Bluetooth),
                    false,
                    1.0,
                    &icons()
                )
                .expect("the toggle is always there");

            let mut ui = simulator(submenu.expect("the open submenu is drawn"));
            assert!(ui.snapshot(&Theme::Dark).is_ok());
        }
    }

    #[test]
    fn another_open_submenu_does_not_open_the_device_list() {
        let data = data(BluetoothState::Active, vec![]);

        let (_, submenu) = data
            .get_quick_setting_button(surface(), Some(SubMenu::Wifi), false, 1.0, &icons())
            .expect("the toggle is always there");

        assert!(submenu.is_none());
    }

    #[test]
    fn an_adapter_that_has_paired_with_nothing_says_so() {
        let data = data(BluetoothState::Active, vec![]);

        let mut ui = simulator(data.bluetooth_menu(surface(), false, 1.0, &icons()));

        assert!(ui.find("No paired devices").is_ok());
    }

    #[test]
    fn every_paired_device_is_named_with_the_deed_it_offers() {
        let data = data(
            BluetoothState::Active,
            vec![device("buds", true, None), device("mouse", false, None)]
        );

        let mut ui = simulator(data.bluetooth_menu(surface(), false, 1.0, &icons()));

        assert!(ui.find("buds").is_ok());
        assert!(ui.find("mouse").is_ok());
        assert!(ui.find("Disconnect").is_ok());
        assert!(ui.find("Connect").is_ok());
    }

    #[test]
    fn pressing_connect_asks_for_that_device() {
        let data = data(BluetoothState::Active, vec![device("mouse", false, None)]);

        let mut ui = simulator(data.bluetooth_menu(surface(), false, 1.0, &icons()));
        let _ = ui.click("Connect").expect("the deed is pressable");

        assert!(ui.into_messages().any(|message| matches!(
            message,
            Message::Bluetooth(BluetoothMessage::ConnectDevice(at))
                if at.as_str() == "/device/mouse"
        )));
    }

    #[test]
    fn pressing_disconnect_asks_for_that_device() {
        let data = data(BluetoothState::Active, vec![device("buds", true, None)]);

        let mut ui = simulator(data.bluetooth_menu(surface(), false, 1.0, &icons()));
        let _ = ui.click("Disconnect").expect("the deed is pressable");

        assert!(ui.into_messages().any(|message| matches!(
            message,
            Message::Bluetooth(BluetoothMessage::DisconnectDevice(at))
                if at.as_str() == "/device/buds"
        )));
    }

    #[test]
    fn a_device_that_reports_its_charge_shows_it() {
        let data = data(BluetoothState::Active, vec![device("buds", true, Some(64))]);

        let mut ui = simulator(data.bluetooth_menu(surface(), false, 1.0, &icons()));

        assert!(ui.find("64%").is_ok());
    }

    #[test]
    fn a_device_that_reports_no_charge_shows_none() {
        let data = data(BluetoothState::Active, vec![device("buds", true, None)]);

        let mut ui = simulator(data.bluetooth_menu(surface(), false, 1.0, &icons()));

        assert!(ui.find("100%").is_err());
        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }

    #[test]
    fn every_step_of_the_charge_is_drawn() {
        for charge in [10, 30, 50, 70, 90] {
            let data = data(
                BluetoothState::Active,
                vec![device("buds", true, Some(charge))]
            );

            let mut ui = simulator(data.bluetooth_menu(surface(), false, 1.0, &icons()));

            assert!(ui.find(format!("{charge}%")).is_ok());
            assert!(ui.snapshot(&Theme::Dark).is_ok());
        }
    }

    #[test]
    fn a_menu_that_can_show_more_carries_the_offer() {
        let data = data(BluetoothState::Active, vec![device("buds", true, None)]);

        let mut ui = simulator(data.bluetooth_menu(surface(), true, 1.0, &icons()));

        assert!(ui.find("More").is_ok());
    }

    #[test]
    fn a_menu_that_holds_everything_offers_nothing_more() {
        let data = data(BluetoothState::Active, vec![device("buds", true, None)]);

        let mut ui = simulator(data.bluetooth_menu(surface(), false, 1.0, &icons()));

        assert!(ui.find("More").is_err());
    }

    #[test]
    fn pressing_more_asks_the_list_to_open_wider() {
        let data = data(BluetoothState::Active, vec![]);

        let mut ui = simulator(data.bluetooth_menu(surface(), true, 1.0, &icons()));
        let _ = ui.click("More").expect("the offer is pressable");

        assert!(
            ui.into_messages()
                .any(|message| matches!(message, Message::Bluetooth(BluetoothMessage::More(_))))
        );
    }
}
