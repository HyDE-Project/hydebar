//! Shared layout helpers for the settings menu.

use iced::{
    Background, Border, Element, Length, Theme,
    widget::{Space, column, container, row}
};

use crate::modules::control_center::state::Message;

pub(super) fn quick_settings_section<'a>(
    buttons: Vec<(Element<'a, Message>, Option<Element<'a, Message>>)>,
    opacity: f32
) -> Element<'a, Message> {
    let mut section = column!().width(Length::Fill).spacing(8);

    let mut before: Option<(Element<'a, Message>, Option<Element<'a, Message>>)> = None;

    for (button, menu) in buttons.into_iter() {
        match before.take() {
            Some((before_button, before_menu)) => {
                section = section.push(row![before_button, button].width(Length::Fill).spacing(8));

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
                .spacing(8)
        );

        if let Some(menu) = before_menu {
            section = section.push(sub_menu_wrapper(menu, opacity));
        }
    }

    section.into()
}

pub(crate) fn sub_menu_wrapper<Msg: 'static>(content: Element<Msg>, opacity: f32) -> Element<Msg> {
    container(content)
        .style(move |theme: &Theme| container::Style {
            background: Background::Color(
                theme
                    .extended_palette()
                    .secondary
                    .strong
                    .color
                    .scale_alpha(opacity)
            )
            .into(),
            border: Border::default().rounded(16),
            ..container::Style::default()
        })
        .padding(16)
        .width(Length::Fill)
        .into()
}
