use iced::{
    Alignment, Element, Theme,
    widget::{Container, container, row}
};

use super::{Message, quick_setting_button};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    },
    services::{
        ServiceEvent,
        upower::{BatteryData, BatteryStatus, PowerProfile, UPowerService}
    },
    utils::{IndicatorState, format_duration}
};

#[derive(Clone, Debug)]
pub enum UPowerMessage {
    Event(Box<ServiceEvent<UPowerService>>),
    TogglePowerProfile
}

impl BatteryData {
    #[must_use]
    pub fn indicator<Message: 'static>(&self, icons: &IconTheme) -> Element<'static, Message> {
        let icon_type = self.get_icon();
        let state = self.get_indicator_state();

        container(
            row!(icon(icons, icon_type), text(format!("{}%", self.capacity)))
                .spacing(scale::icon_gap())
                .align_y(Alignment::Center)
        )
        .style(move |theme: &Theme| container::Style {
            text_color: Some(match state {
                IndicatorState::Success => theme.palette().success,
                IndicatorState::Danger => theme.palette().danger,
                _ => theme.palette().text
            }),
            ..Default::default()
        })
        .into()
    }

    #[must_use]
    pub fn settings_indicator<'a, Message: 'static>(
        &self,
        icons: &IconTheme
    ) -> Container<'a, Message> {
        let state = self.get_indicator_state();

        container({
            let battery_info = container(
                row!(
                    icon(icons, self.get_icon()),
                    text(format!("{}%", self.capacity))
                )
                .spacing(scale::icon_gap())
            )
            .style(move |theme: &Theme| container::Style {
                text_color: Some(match state {
                    IndicatorState::Success => theme.palette().success,
                    IndicatorState::Danger => theme.palette().danger,
                    _ => theme.palette().text
                }),
                ..Default::default()
            });

            match self.status {
                BatteryStatus::Charging(remaining) if self.capacity < 95 => row!(
                    battery_info,
                    text(format!("Full in {}", format_duration(&remaining)))
                )
                .spacing(scale::scaled(16.0)),
                BatteryStatus::Discharging(remaining) if self.capacity < 95 => row!(
                    battery_info,
                    text(format!("Empty in {}", format_duration(&remaining)))
                )
                .spacing(scale::scaled(16.0)),
                _ => row!(battery_info)
            }
        })
        .padding([scale::scaled(8.0), scale::scaled(4.0)])
    }
}

impl PowerProfile {
    #[must_use]
    pub fn indicator<Message: 'static>(
        &self,
        icons: &IconTheme
    ) -> Option<Element<'static, Message>> {
        match self {
            Self::Balanced | Self::Unknown => None,
            Self::Performance => Some(
                container(icon(icons, Icons::Performance))
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.palette().danger),
                        ..Default::default()
                    })
                    .into()
            ),
            Self::PowerSaver => Some(
                container(icon(icons, Icons::PowerSaver))
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.palette().success),
                        ..Default::default()
                    })
                    .into()
            )
        }
    }

    #[must_use]
    pub fn get_quick_setting_button(
        &self,
        opacity: f32,
        icons: &IconTheme
    ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
        if matches!(self, Self::Unknown) {
            None
        } else {
            Some((
                quick_setting_button(
                    icons,
                    (*self).into(),
                    match self {
                        Self::Balanced => "Balanced",
                        Self::Performance => "Performance",
                        Self::PowerSaver => "Power Saver",
                        Self::Unknown => ""
                    }
                    .to_string(),
                    None,
                    true,
                    Message::UPower(UPowerMessage::TogglePowerProfile),
                    None,
                    opacity
                ),
                None
            ))
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::time::Duration;

    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Probe {}

    fn icons() -> IconTheme {
        IconTheme::default()
    }

    fn battery(capacity: i64, status: BatteryStatus) -> BatteryData {
        BatteryData {
            capacity,
            status
        }
    }

    #[test]
    fn the_bar_indicator_states_the_charge_as_a_percentage() {
        let data = battery(72, BatteryStatus::Full);
        let mut ui = simulator(data.indicator::<Probe>(&icons()));

        assert!(ui.find("72%").is_ok());
        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }

    #[test]
    fn a_charging_battery_reads_as_success_and_a_low_one_as_danger() {
        let charging = battery(50, BatteryStatus::Charging(Duration::from_mins(10)));
        let low = battery(10, BatteryStatus::Discharging(Duration::from_mins(10)));
        let resting = battery(50, BatteryStatus::Full);

        assert!(matches!(
            charging.get_indicator_state(),
            IndicatorState::Success
        ));
        assert!(matches!(low.get_indicator_state(), IndicatorState::Danger));
        assert!(matches!(
            resting.get_indicator_state(),
            IndicatorState::Normal
        ));
    }

    #[test]
    fn every_indicator_state_is_drawn() {
        for data in [
            battery(50, BatteryStatus::Charging(Duration::from_mins(1))),
            battery(10, BatteryStatus::Discharging(Duration::from_mins(1))),
            battery(50, BatteryStatus::Full)
        ] {
            let mut ui = simulator(data.indicator::<Probe>(&icons()));

            assert!(ui.snapshot(&Theme::Dark).is_ok());
        }
    }

    #[test]
    fn a_charging_battery_says_when_it_will_be_full() {
        let data = battery(40, BatteryStatus::Charging(Duration::from_hours(1)));
        let mut ui = simulator(Element::<Probe>::from(data.settings_indicator(&icons())));

        assert!(ui.find("Full in 1h  0m").is_ok());
        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }

    #[test]
    fn a_discharging_battery_says_when_it_will_be_empty() {
        let data = battery(40, BatteryStatus::Discharging(Duration::from_mins(30)));
        let mut ui = simulator(Element::<Probe>::from(data.settings_indicator(&icons())));

        assert!(ui.find("Empty in 30m").is_ok());
    }

    #[test]
    fn a_nearly_full_battery_states_no_remaining_time() {
        let data = battery(96, BatteryStatus::Charging(Duration::from_mins(1)));
        let mut ui = simulator(Element::<Probe>::from(data.settings_indicator(&icons())));

        assert!(ui.find("96%").is_ok());
        assert!(ui.find("Full in  1m").is_err());
    }

    #[test]
    fn a_full_battery_states_no_remaining_time() {
        let data = battery(100, BatteryStatus::Full);
        let mut ui = simulator(Element::<Probe>::from(data.settings_indicator(&icons())));

        assert!(ui.find("100%").is_ok());
        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }

    #[test]
    fn only_the_profiles_that_depart_from_balance_carry_an_indicator() {
        assert!(
            PowerProfile::Balanced
                .indicator::<Probe>(&icons())
                .is_none()
        );
        assert!(PowerProfile::Unknown.indicator::<Probe>(&icons()).is_none());
        assert!(
            PowerProfile::Performance
                .indicator::<Probe>(&icons())
                .is_some()
        );
        assert!(
            PowerProfile::PowerSaver
                .indicator::<Probe>(&icons())
                .is_some()
        );
    }

    #[test]
    fn a_profile_indicator_is_drawn_in_the_colour_of_what_it_costs() {
        for profile in [PowerProfile::Performance, PowerProfile::PowerSaver] {
            let indicator = profile
                .indicator::<Probe>(&icons())
                .expect("the profile carries an indicator");
            let mut ui = simulator(indicator);

            assert!(ui.snapshot(&Theme::Dark).is_ok());
        }
    }

    #[test]
    fn an_unknown_profile_offers_no_quick_setting() {
        assert!(
            PowerProfile::Unknown
                .get_quick_setting_button(1.0, &icons())
                .is_none()
        );
    }

    #[test]
    fn every_known_profile_offers_a_named_quick_setting() {
        for (profile, name) in [
            (PowerProfile::Balanced, "Balanced"),
            (PowerProfile::Performance, "Performance"),
            (PowerProfile::PowerSaver, "Power Saver")
        ] {
            let (button, submenu) = profile
                .get_quick_setting_button(1.0, &icons())
                .expect("a known profile offers a quick setting");

            assert!(submenu.is_none());

            let mut ui = simulator(button);
            assert!(ui.find(name.to_owned()).is_ok(), "{name} is named");
        }
    }

    #[test]
    fn pressing_the_profile_quick_setting_asks_to_cycle_it() {
        let (button, _) = PowerProfile::Balanced
            .get_quick_setting_button(1.0, &icons())
            .expect("a known profile offers a quick setting");

        let mut ui = simulator(button);
        let _ = ui
            .click("Balanced")
            .expect("the quick setting is pressable");

        assert!(
            ui.into_messages().any(|message| matches!(
                message,
                Message::UPower(UPowerMessage::TogglePowerProfile)
            ))
        );
    }
}
