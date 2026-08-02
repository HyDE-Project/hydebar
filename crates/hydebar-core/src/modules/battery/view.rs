//! Drawing of the battery entry: profile glyph, icon and painted percentage.

use iced::Element;

use super::{Battery, IndicatorState};

impl Battery {
    /// The bar entry: the power profile glyph and the painted percentage.
    ///
    /// Rendered by the module itself, so the bar layer holds no battery
    /// drawing of its own.
    #[must_use]
    pub fn bar_view<M: 'static>(
        &self,
        config: &hydebar_proto::config::BatteryModuleConfig,
        icons: &crate::components::icons::IconTheme
    ) -> Option<(
        Element<'static, M>,
        Option<crate::modules::OnModulePress<M>>
    )> {
        use iced::{
            Alignment, Theme,
            widget::{container, row}
        };

        use crate::components::{icons::icon, scale};

        let data = self.data.as_ref()?;

        let mut segments: Vec<Element<'static, M>> = Vec::with_capacity(2);

        if config.show_power_profile {
            segments.push(
                container(icon(icons, data.power_profile.into()))
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.palette().primary),
                        ..Default::default()
                    })
                    .into()
            );
        }

        let mut content = row![icon(icons, data.icon.into())]
            .align_y(Alignment::Center)
            .spacing(scale::icon_gap());

        if config.show_percentage {
            content = content.push(self.percent_element(scale::base()));
        }

        let indicator_state = data.indicator_state;
        segments.push(
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
        );

        let press = config.open_settings_on_click.then(|| {
            crate::modules::OnModulePress::ToggleMenu(crate::menu::MenuType::ControlCenter)
        });

        Some((
            row(segments)
                .align_y(Alignment::Center)
                .spacing(scale::icon_gap())
                .into(),
            press
        ))
    }

    /// The percentage as the bar shows it, painted by the indicator state.
    #[must_use]
    pub fn percent_element<M: 'static>(&self, size: f32) -> Element<'static, M> {
        use crate::components::crossfade::Role;

        let role = match self.data.as_ref().map(|data| data.indicator_state) {
            Some(IndicatorState::Success) => Role::Success,
            Some(IndicatorState::Warning) => Role::Warning,
            Some(IndicatorState::Danger) => Role::Danger,
            _ => Role::Text
        };

        self.shown.element_role(size, role)
    }
}
