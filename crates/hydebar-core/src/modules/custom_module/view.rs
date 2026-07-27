//! Bar rendering for modules driven by an external command.

use iced::{
    Element, Length, Theme,
    mouse::Cursor,
    widget::{
        Stack, canvas,
        canvas::{Cache, Geometry, Path, Program},
        container, row, text, tooltip
    }
};

use super::Custom;
use crate::{
    components::icons::{Icons, icon, icon_raw},
    config::CustomModuleDef
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

/// Builds the bar content for a custom module.
pub(super) fn render<M>(module: &Custom, config: &CustomModuleDef) -> Element<'static, M>
where
    M: 'static + Clone
{
    let mut icon_element = config
        .icon
        .as_ref()
        .map_or_else(|| icon(Icons::None), |text| icon_raw(text.clone()));

    if let Some(icons_map) = &config.icons {
        for (re, icon_str) in icons_map {
            if re.is_match(&module.data.alt) {
                icon_element = icon_raw(icon_str.clone());
                break;
            }
        }
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

    let row_content: Element<'static, M> = if let Some(text_element) = maybe_text_element {
        row![icon_with_alert, text_element].spacing(8).into()
    } else {
        icon_with_alert
    };

    match module.data.tooltip.as_ref() {
        Some(hint) if !hint.is_empty() && module.last_error.is_none() => tooltip(
            row_content,
            container(text(hint.clone())).padding([4, 8]),
            tooltip::Position::Bottom
        )
        .into(),
        _ => row_content
    }
}
