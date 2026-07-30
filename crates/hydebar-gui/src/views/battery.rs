/// Battery module view layer - Pure rendering, no business logic
use hydebar_core::components::text::text;
use hydebar_core::{
    components::{
        icons::{IconTheme, icon},
        scale
    },
    config::BatteryModuleConfig,
    modules::battery::{BatteryData, IndicatorState}
};
use iced::{
    Alignment, Element, Theme,
    widget::{container, row}
};

use crate::app::Message;

/// Render battery indicator for the bar
pub fn render_battery_indicator(
    data: &BatteryData,
    config: &BatteryModuleConfig,
    icons: &IconTheme
) -> Element<'static, Message> {
    let mut content = row![icon(icons, data.icon.into())]
        .align_y(Alignment::Center)
        .spacing(scale::icon_gap());

    if config.show_percentage {
        content = content.push(text(format!("{}%", data.capacity)));
    }

    let indicator_state = data.indicator_state;
    container(content)
        .style(move |theme: &Theme| container::Style {
            text_color: Some(match indicator_state {
                IndicatorState::Success => theme.palette().success,
                IndicatorState::Warning => theme.palette().warning,
                IndicatorState::Danger => theme.palette().danger,
                IndicatorState::Normal => theme.palette().text
            }),
            ..Default::default()
        })
        .into()
}

/// Render power profile indicator
pub fn render_power_profile(data: &BatteryData, icons: &IconTheme) -> Element<'static, Message> {
    container(icon(icons, data.power_profile.into()))
        .style(|theme: &Theme| container::Style {
            text_color: Some(theme.palette().primary),
            ..Default::default()
        })
        .into()
}

/// Render complete battery widget (indicator + profile)
pub fn render_battery(
    data: &BatteryData,
    config: &BatteryModuleConfig,
    icons: &IconTheme
) -> Element<'static, Message> {
    let mut segments = vec![];

    if config.show_power_profile {
        segments.push(render_power_profile(data, icons));
    }

    segments.push(render_battery_indicator(data, config, icons));

    row(segments)
        .align_y(Alignment::Center)
        .spacing(scale::icon_gap())
        .into()
}
