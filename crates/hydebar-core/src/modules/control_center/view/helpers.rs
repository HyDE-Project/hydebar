//! Shared layout helpers for the settings menu.

use iced::{
    Background, Border, Element, Length, Theme,
    widget::{Space, column, container, row}
};

use crate::{
    components::scale, modules::control_center::state::Message, style::darken_color
};

/// How much darker an unfolded zone is than the menu it sits in.
///
/// Darker rather than lighter on purpose: the zone is a recess the
/// extra facts sit in, and a lighter shade reads as a raised
/// control instead.
const SUB_MENU_DARKENING: f32 = 0.25;

pub(super) fn quick_settings_section<'a>(
    buttons: Vec<(Element<'a, Message>, Option<Element<'a, Message>>)>,
    opacity: f32
) -> Element<'a, Message> {
    let mut section = column!().width(Length::Fill).spacing(scale::scaled(8.0));

    let mut before: Option<(Element<'a, Message>, Option<Element<'a, Message>>)> = None;

    for (button, menu) in buttons {
        match before.take() {
            Some((before_button, before_menu)) => {
                section = section.push(
                    row![before_button, button]
                        .width(Length::Fill)
                        .spacing(scale::scaled(8.0))
                );

                if let Some(menu) = before_menu {
                    section = section.push(sub_menu_wrapper(menu, opacity));
                }

                if let Some(menu) = menu {
                    section = section.push(sub_menu_wrapper(menu, opacity));
                }
            }
            _ => {
                before = Some((button, menu));
            }
        }
    }

    if let Some((before_button, before_menu)) = before.take() {
        section = section.push(
            row![before_button, Space::new().width(Length::Fill)]
                .width(Length::Fill)
                .spacing(scale::scaled(8.0))
        );

        if let Some(menu) = before_menu {
            section = section.push(sub_menu_wrapper(menu, opacity));
        }
    }

    section.into()
}

pub fn sub_menu_wrapper<Msg: 'static>(
    content: Element<Msg>,
    opacity: f32
) -> Element<Msg> {
    container(content)
        .style(move |theme: &Theme| container::Style {
            background: Background::Color(
                darken_color(theme.palette().background, SUB_MENU_DARKENING)
                    .scale_alpha(opacity)
            )
            .into(),
            border: Border::default().rounded(scale::scaled(16.0)),
            ..container::Style::default()
        })
        .padding(scale::scaled(16.0))
        .width(Length::Fill)
        .into()
}
