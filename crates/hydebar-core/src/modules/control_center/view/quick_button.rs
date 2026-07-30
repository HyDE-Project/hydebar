//! Quick setting toggle button.

use iced::{
    Alignment, Element, Length, Padding,
    alignment::{Horizontal, Vertical},
    widget::{Column, Row, button, container, row, text}
};

use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale
    },
    modules::control_center::state::SubMenu,
    style::{quick_settings_button_style, quick_settings_submenu_button_style}
};

pub fn quick_setting_button<'a, Msg: Clone + 'static>(
    icons: &IconTheme,
    icon_type: Icons,
    title: String,
    subtitle: Option<String>,
    active: bool,
    on_press: Msg,
    with_submenu: Option<(SubMenu, Option<SubMenu>, Msg)>,
    opacity: f32
) -> Element<'a, Msg> {
    let main_content = row!(
        icon(icons, icon_type).size(scale::scaled(20.0)),
        Column::new()
            .push(text(title).size(scale::scaled(12.0)))
            .push_maybe(subtitle.map(|s| text(s).size(scale::scaled(10.0))))
            .spacing(4)
    )
    .spacing(8)
    .padding(Padding::ZERO.left(4))
    .width(Length::Fill)
    .align_y(Alignment::Center);

    button(
        Row::new()
            .push(main_content)
            .push_maybe(with_submenu.map(|(menu_type, submenu, msg)| {
                button(
                    container(icon(
                        icons,
                        if Some(menu_type) == submenu {
                            Icons::Close
                        } else {
                            Icons::RightChevron
                        }
                    ))
                    .align_y(Vertical::Center)
                    .align_x(Horizontal::Center)
                )
                .padding([4, if Some(menu_type) == submenu { 9 } else { 12 }])
                .style(quick_settings_submenu_button_style(active, opacity))
                .width(Length::Shrink)
                .height(Length::Shrink)
                .on_press(msg)
            }))
            .spacing(4)
            .align_y(Alignment::Center)
            .height(Length::Fill)
    )
    .padding([4, 8])
    .on_press(on_press)
    .height(Length::Fill)
    .width(Length::Fill)
    .style(quick_settings_button_style(active, opacity))
    .width(Length::Fill)
    .height(Length::Fixed(50.))
    .into()
}
