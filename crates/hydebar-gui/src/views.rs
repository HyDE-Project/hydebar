//! Widget layer: views composed from the modules they draw.

pub mod battery {
    /// Battery module view layer - Pure rendering, no business logic
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
        icons: &IconTheme,
        percent: Element<'static, Message>
    ) -> Element<'static, Message> {
        let mut content = row![icon(icons, data.icon.into())]
            .align_y(Alignment::Center)
            .spacing(scale::icon_gap());

        if config.show_percentage {
            content = content.push(percent);
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
    pub fn render_power_profile(
        data: &BatteryData,
        icons: &IconTheme
    ) -> Element<'static, Message> {
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
        icons: &IconTheme,
        percent: Element<'static, Message>
    ) -> Element<'static, Message> {
        let mut segments = vec![];

        if config.show_power_profile {
            segments.push(render_power_profile(data, icons));
        }

        segments.push(render_battery_indicator(data, config, icons, percent));

        row(segments)
            .align_y(Alignment::Center)
            .spacing(scale::icon_gap())
            .into()
    }
}

pub mod tray {
    //! Tray module view layer: one bar icon per status notifier item.

    use hydebar_core::{
        components::{
            icons::{IconTheme, Icons, icon},
            scale
        },
        menu::MenuType,
        modules::tray::TrayModule,
        position_button::position_button,
        services::tray::TrayIcon,
        style::ghost_button_style
    };
    use iced::{
        Alignment, Element, Length, SurfaceId as Id,
        widget::{Row, image, svg}
    };

    use crate::app::Message;

    /// Share of the themed icon size a tray image is drawn at.
    ///
    /// The font glyphs ink about this share of their stated size; a trimmed
    /// image fills its box edge to edge, so drawn at the full size it stands
    /// a head taller than every glyph beside it. Measured against the bar,
    /// not derived.
    const GLYPH_MATCH: f32 = 0.85;

    /// Renders the tray strip, or nothing while no application is registered.
    ///
    /// Each icon is its own positioned button so the menu it toggles opens
    /// under the icon that was pressed, not under the strip as a whole.
    pub fn render_tray(
        module: &TrayModule,
        id: Id,
        opacity: f32,
        icons: &IconTheme
    ) -> Option<Element<'static, Message>> {
        let items = module.service.as_ref().filter(|s| !s.data.is_empty())?;
        let size = icons.size().unwrap_or_else(scale::base) * GLYPH_MATCH;

        Some(
            Row::with_children(items.data.iter().map(|item| {
                let content: Element<'static, Message> = match &item.icon {
                    Some(TrayIcon::Image(handle)) => image(handle.clone())
                        .height(Length::Fixed(size))
                        .width(Length::Fixed(size))
                        .into(),
                    Some(TrayIcon::Svg(handle)) => svg(handle.clone())
                        .height(Length::Fixed(size))
                        .width(Length::Fixed(size))
                        .into(),
                    None => icon(icons, Icons::Point).into()
                };

                let name = item.name.clone();

                position_button(content)
                    .padding([scale::scaled(2.0), scale::scaled(4.0)])
                    .style(ghost_button_style(opacity))
                    .on_press_with_position(move |button_ui_ref| {
                        Message::ToggleMenu(MenuType::Tray(name.clone()), id, button_ui_ref)
                    })
                    .into()
            }))
            .align_y(Alignment::Center)
            .spacing(scale::icon_gap())
            .into()
        )
    }
}
