//! Bar rendering for modules driven by an external command.

use iced::{
    Color, Element, Length, Theme,
    mouse::Cursor,
    widget::{
        Stack, canvas,
        canvas::{Cache, Geometry, Path, Program},
        container, row
    }
};

use super::Custom;
use crate::{
    components::{
        icons::{IconTheme, Icons, icon, icon_raw},
        text::text
    },
    config::{Appearance, CustomModuleDef}
};

/// Small circle drawn over the icon while the module is in an alert state.
///
/// Carries its radius so the dot follows the themed sizes instead of
/// staying two pixels on every screen.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AlertIndicator {
    radius: f32
}

impl<Message> Program<Message> for AlertIndicator {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: Cursor
    ) -> Vec<Geometry> {
        let cache = Cache::new();

        vec![cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();
            let circle = Path::circle(center, self.radius);
            frame.fill(&circle, theme.palette().danger);
        })]
    }
}

/// Diameter of the alert dot, in em of the themed font.
const ALERT_DOT_EM: f32 = 0.5;

/// Resolves the color a module paints itself with for the state it reports.
///
/// The alternate state carries the bucket a listener assigns to its
/// reading, so a temperature readout can shade itself from cold to
/// critical the way the equivalent Waybar stylesheet does.
pub(super) fn state_color(module: &Custom, config: &CustomModuleDef) -> Option<Color> {
    #[expect(
        clippy::mutable_key_type,
        reason = "RegexCfg hashes and compares by its pattern text; the regex \
                  interior mutability never feeds the map key"
    )]
    let colors = config.colors.as_ref()?;

    colors
        .iter()
        .find_map(|(pattern, color)| pattern.is_match(&module.data.alt).then(|| color.get_base()))
}

/// Builds the bar content for a custom module.
///
/// The gap between the icon and its text is derived from the themed font
/// size carried by `appearance` instead of being fixed in pixels.
pub(super) fn render<M>(
    module: &Custom,
    config: &CustomModuleDef,
    appearance: &Appearance,
    icons: &IconTheme
) -> Element<'static, M>
where
    M: 'static + Clone
{
    let state_color = state_color(module, config);

    let mut icon_element = config.icon.as_ref().map_or_else(
        || icon(icons, Icons::None),
        |glyph| icon_raw(glyph.trim().to_owned())
    );

    if let Some(icons_map) = &config.icons {
        for (re, icon_str) in icons_map {
            if re.is_match(&module.data.alt) {
                icon_element = icon_raw(icon_str.trim().to_owned());
                break;
            }
        }
    }

    if let Some(color) = state_color {
        icon_element = icon_element.color(color);
    }

    let padded_icon_container = container(icon_element);

    let show_alert = config
        .alert
        .as_ref()
        .is_some_and(|re| re.is_match(&module.data.alt))
        || module.last_error.is_some();

    let icon_with_alert: Element<'static, M> = if show_alert {
        let dot = appearance.spacing(ALERT_DOT_EM);
        let alert_canvas = canvas(AlertIndicator {
            radius: dot / 2.0
        })
        .width(Length::Fixed(dot))
        .height(Length::Fixed(dot));

        let alert_indicator_container = container(alert_canvas)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Top);

        Stack::new()
            .push(padded_icon_container)
            .push(alert_indicator_container)
            .into()
    } else {
        padded_icon_container.into()
    };

    let maybe_text_element = module.last_error.as_ref().map_or_else(
        || {
            module.data.text.as_ref().and_then(|text_content| {
                let trimmed = text_content.trim();

                if trimmed.is_empty() {
                    None
                } else {
                    Some(text(trimmed.to_owned()))
                }
            })
        },
        |error| Some(text(error.to_display_message()))
    );

    let maybe_text_element = maybe_text_element.map(|text_element| match state_color {
        Some(color) => text_element.color(color),
        None => text_element
    });

    let row_content: Element<'static, M> = if let Some(text_element) = maybe_text_element {
        row![icon_with_alert, text_element]
            .spacing(appearance.icon_label_gap())
            .into()
    } else {
        icon_with_alert
    };

    row_content
}

impl Custom {
    /// Text the module asks the bar to show while the pointer rests on it.
    ///
    /// The bar surface is only as tall as the bar, so the hint cannot be
    /// drawn as an overlay next to the module without covering it.
    /// It is handed to the tooltip surface instead, which the
    /// compositor lays out beside the bar.
    #[must_use]
    pub fn tooltip(&self) -> Option<&str> {
        match self.data.tooltip.as_deref() {
            Some(hint) if !hint.is_empty() && self.last_error.is_none() => Some(hint),
            _ => None
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{
        super::{data::CustomListenData, error::CustomCommandError},
        *
    };

    fn module_in_state(alt: &str) -> Custom {
        let mut module = Custom::default();
        module.data = CustomListenData {
            alt: alt.to_owned(),
            ..CustomListenData::default()
        };
        module
    }

    fn config_with_colors() -> CustomModuleDef {
        toml::from_str(
            r##"
            name = "battery"
            command = ""

            [colors]
            "^charging$" = "#ff8800"
            "##
        )
        .expect("custom module definition")
    }

    #[test]
    fn a_module_without_a_color_map_keeps_the_default_paint() {
        let module = module_in_state("charging");

        assert!(state_color(&module, &CustomModuleDef::default()).is_none());
    }

    #[test]
    fn the_matching_state_picks_its_configured_color() {
        let module = module_in_state("charging");

        assert_eq!(
            state_color(&module, &config_with_colors()),
            Some(Color::from_rgb8(0xff, 0x88, 0x00))
        );
    }

    #[test]
    fn an_unmatched_state_keeps_the_default_paint() {
        let module = module_in_state("full");

        assert!(state_color(&module, &config_with_colors()).is_none());
    }

    #[test]
    fn a_tooltip_shows_only_while_the_listener_is_healthy() {
        let mut module = Custom::default();
        module.data = CustomListenData {
            tooltip: Some("Battery at 42%".to_owned()),
            ..CustomListenData::default()
        };

        assert_eq!(module.tooltip(), Some("Battery at 42%"));

        module.last_error = Some(CustomCommandError::ChannelClosed);

        assert!(module.tooltip().is_none());
    }

    #[test]
    fn an_empty_hint_is_no_tooltip() {
        let mut module = Custom::default();

        assert!(module.tooltip().is_none());

        module.data = CustomListenData {
            tooltip: Some(String::new()),
            ..CustomListenData::default()
        };

        assert!(module.tooltip().is_none());
    }
}
