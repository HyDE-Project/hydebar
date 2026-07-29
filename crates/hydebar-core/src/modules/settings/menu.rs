//! Rendering of the settings window.
//!
//! Every row reads its current value from the running configuration, so the
//! window shows the truth after a reload instead of a copy that drifted.

use iced::{
    Alignment, Element, Length, Theme,
    widget::{Column, Row, Space, button, container, text}
};

use super::{Message, Settings};
use crate::{
    components::icons::{IconTheme, Icons, icon},
    config::{Appearance, AppearanceStyle, BarLayer, Config, DEFAULT_FONT_SIZE, Position},
    style::settings_button_style
};

/// Height the bar falls back to while the configuration names none.
const FALLBACK_HEIGHT: f32 = 34.0;

/// Renders a row of mutually exclusive choices, the active one highlighted.
fn choice_row<'a, T>(
    label: &'a str,
    choices: Vec<(&'a str, T, bool)>,
    to_message: impl Fn(T) -> Message + 'a,
    opacity: f32
) -> Element<'a, Message>
where
    T: Clone + 'a
{
    let mut buttons = Row::new().spacing(8);

    for (name, choice, active) in choices {
        buttons = buttons.push(
            button(text(name))
                .padding([6, 12])
                .on_press(to_message(choice))
                .style(move |theme: &Theme, status| {
                    let mut style = settings_button_style(opacity)(theme, status);

                    if active {
                        style.text_color = theme.extended_palette().primary.base.color;
                    }

                    style
                })
        );
    }

    Row::new()
        .push(text(label).width(Length::Fill))
        .push(buttons)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

/// Renders a row stepping a number down and up around its current value.
fn stepper_row<'a>(
    label: &'a str,
    current: String,
    down: Message,
    up: Message,
    opacity: f32
) -> Element<'a, Message> {
    Row::new()
        .push(text(label).width(Length::Fill))
        .push(
            button(text("−"))
                .padding([6, 12])
                .on_press(down)
                .style(settings_button_style(opacity))
        )
        .push(
            container(text(current))
                .padding([6, 8])
                .align_x(Alignment::Center)
        )
        .push(
            button(text("+"))
                .padding([6, 12])
                .on_press(up)
                .style(settings_button_style(opacity))
        )
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

impl Settings {
    /// Renders the settings window against the running `config`.
    pub fn menu_view<'a>(
        &self,
        config: &'a Config,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'a, Message> {
        let appearance: &Appearance = &config.appearance;
        let height = appearance.height.unwrap_or(FALLBACK_HEIGHT);
        let font_size = appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);

        let header = Row::new()
            .push(icon(icons, Icons::Settings))
            .push(text("Bar settings").width(Length::Fill))
            .spacing(8)
            .align_y(Alignment::Center);

        Column::new()
            .push(header)
            .push(Space::new().height(4))
            .push(choice_row(
                "Position",
                vec![
                    ("Top", Position::Top, config.position == Position::Top),
                    (
                        "Bottom",
                        Position::Bottom,
                        config.position == Position::Bottom
                    ),
                ],
                Message::SetPosition,
                opacity
            ))
            .push(choice_row(
                "Layer",
                vec![
                    ("Bottom", BarLayer::Bottom, config.layer == BarLayer::Bottom),
                    ("Top", BarLayer::Top, config.layer == BarLayer::Top),
                    (
                        "Overlay",
                        BarLayer::Overlay,
                        config.layer == BarLayer::Overlay
                    ),
                ],
                Message::SetLayer,
                opacity
            ))
            .push(choice_row(
                "Style",
                vec![
                    (
                        "Islands",
                        AppearanceStyle::Islands,
                        appearance.style == AppearanceStyle::Islands
                    ),
                    (
                        "Solid",
                        AppearanceStyle::Solid,
                        appearance.style == AppearanceStyle::Solid
                    ),
                    (
                        "Gradient",
                        AppearanceStyle::Gradient,
                        appearance.style == AppearanceStyle::Gradient
                    ),
                ],
                Message::SetStyle,
                opacity
            ))
            .push(stepper_row(
                "Height",
                format!("{height:.0}"),
                Message::SetHeight(Self::height_below(height)),
                Message::SetHeight(Self::height_above(height)),
                opacity
            ))
            .push(stepper_row(
                "Font size",
                format!("{font_size:.0}"),
                Message::SetFontSize(Self::font_size_below(font_size)),
                Message::SetFontSize(Self::font_size_above(font_size)),
                opacity
            ))
            .push(stepper_row(
                "Opacity",
                format!("{:.2}", appearance.opacity),
                Message::SetOpacity(Self::opacity_below(appearance.opacity)),
                Message::SetOpacity(Self::opacity_above(appearance.opacity)),
                opacity
            ))
            .push(choice_row(
                "Follow HyDE theme",
                vec![
                    ("On", true, appearance.follow_hyde),
                    ("Off", false, !appearance.follow_hyde),
                ],
                Message::SetFollowHyde,
                opacity
            ))
            .width(Length::Fill)
            .spacing(12)
            .into()
    }
}
