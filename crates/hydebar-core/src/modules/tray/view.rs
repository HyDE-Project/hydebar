//! Rendering of tray icons and their menus.

use iced::{
    Element, Length,
    widget::{Column, Row, button, row, rule, toggler}
};

use super::super::tray::{TrayMessage, TrayModule};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        text::text
    },
    services::tray::dbus::{Layout, LayoutProps},
    style::ghost_button_style
};

impl TrayModule {
    pub fn menu_view(
        &self,
        name: &'_ str,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, TrayMessage> {
        match self
            .service
            .as_ref()
            .and_then(|service| service.data.iter().find(|item| item.name == name))
        {
            Some(item) => Column::with_children(
                item.menu
                    .2
                    .iter()
                    .map(|menu| self.menu_voice(name, menu, opacity, icons))
            )
            .spacing(8)
            .into(),
            _ => Row::new().into()
        }
    }

    pub(super) fn menu_voice(
        &self,
        name: &str,
        layout: &Layout,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, TrayMessage> {
        match &layout.1 {
            LayoutProps {
                label: Some(label),
                toggle_type: Some(toggle_type),
                toggle_state: Some(state),
                ..
            } if toggle_type == "checkmark" => toggler(*state > 0)
                .label(label.replace("_", "").to_owned())
                .on_toggle({
                    let name = name.to_owned();
                    let id = layout.0;

                    move |_| TrayMessage::MenuSelected(name.to_owned(), id)
                })
                .width(Length::Fill)
                .into(),
            LayoutProps {
                children_display: Some(display),
                label: Some(label),
                ..
            } if display == "submenu" => {
                let is_open = self.submenus.contains(&layout.0);
                Column::new()
                    .push(
                        button(row!(
                            text(label.replace("_", "").to_owned()).width(Length::Fill),
                            icon(
                                icons,
                                if is_open {
                                    Icons::MenuOpen
                                } else {
                                    Icons::MenuClosed
                                }
                            )
                        ))
                        .style(ghost_button_style(opacity))
                        .padding([8, 8])
                        .on_press(TrayMessage::ToggleSubmenu(layout.0))
                        .width(Length::Fill)
                    )
                    .push_maybe(if is_open {
                        Some(
                            Column::with_children(
                                layout
                                    .2
                                    .iter()
                                    .map(|menu| self.menu_voice(name, menu, opacity, icons))
                                    .collect::<Vec<_>>()
                            )
                            .padding(iced::Padding {
                                top:    0.0,
                                right:  0.0,
                                bottom: 0.0,
                                left:   16.0
                            })
                            .spacing(4)
                        )
                    } else {
                        None
                    })
                    .into()
            }
            LayoutProps {
                label: Some(label), ..
            } => button(text(label.replace("_", "")))
                .style(ghost_button_style(opacity))
                .on_press(TrayMessage::MenuSelected(name.to_owned(), layout.0))
                .width(Length::Fill)
                .padding([8, 8])
                .into(),
            LayoutProps {
                type_: Some(t), ..
            } if t == "separator" => rule::horizontal(1).into(),
            _ => Row::new().into()
        }
    }
}
