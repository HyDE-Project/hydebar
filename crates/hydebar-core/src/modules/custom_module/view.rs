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
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AlertIndicator;

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
            let radius = 2.0;
            let circle = Path::circle(center, radius);
            frame.fill(&circle, theme.palette().danger);
        })]
    }
}

/// Resolves the color a module paints itself with for the state it reports.
///
/// The alternate state carries the bucket a listener assigns to its reading,
/// so a temperature readout can shade itself from cold to critical the way the
/// equivalent Waybar stylesheet does.
pub(super) fn state_color(module: &Custom, config: &CustomModuleDef) -> Option<Color> {
    let colors = config.colors.as_ref()?;

    colors
        .iter()
        .find_map(|(pattern, color)| pattern.is_match(&module.data.alt).then(|| color.get_base()))
}

/// Builds the bar content for a custom module.
///
/// The gap between the icon and its text is derived from the themed font size
/// carried by `appearance` instead of being fixed in pixels.
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

    let mut icon_element = config
        .icon
        .as_ref()
        .map_or_else(|| icon(icons, Icons::None), |text| icon_raw(text.clone()));

    if let Some(icons_map) = &config.icons {
        for (re, icon_str) in icons_map {
            if re.is_match(&module.data.alt) {
                icon_element = icon_raw(icon_str.clone());
                break;
            }
        }
    }

    if let Some(color) = state_color {
        icon_element = icon_element.color(color);
    }

    let padded_icon_container = container(icon_element).padding([0, 1]);

    let mut show_alert = false;
    if let Some(re) = &config.alert
        && re.is_match(&module.data.alt)
    {
        show_alert = true;
    }

    if module.last_error.is_some() {
        show_alert = true;
    }

    let icon_with_alert: Element<'static, M> = if show_alert {
        let alert_canvas = canvas(AlertIndicator)
            .width(Length::Fixed(5.0))
            .height(Length::Fixed(5.0));

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

    let maybe_text_element = if let Some(error) = &module.last_error {
        Some(text(error.to_display_message()))
    } else {
        module.data.text.as_ref().and_then(|text_content| {
            if text_content.is_empty() {
                None
            } else {
                Some(text(text_content.clone()))
            }
        })
    };

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
    /// The bar surface is only as tall as the bar, so the hint cannot be drawn
    /// as an overlay next to the module without covering it. It is handed to
    /// the tooltip surface instead, which the compositor lays out beside the
    /// bar.
    pub fn tooltip(&self) -> Option<&str> {
        match self.data.tooltip.as_deref() {
            Some(hint) if !hint.is_empty() && self.last_error.is_none() => Some(hint),
            _ => None
        }
    }
}
