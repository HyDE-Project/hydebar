//! Device lists behind the sliders: every sink and source port, the
//! active one highlighted.

use iced::{
    Alignment, Element, Length, SurfaceId as Id, Theme,
    widget::{Column, button, column, container, row, rule}
};

use super::{super::Message, AudioMessage};
use crate::{
    components::{
        icons::{IconTheme, icon},
        scale,
        text::text
    },
    services::audio::{AudioData, DeviceType},
    style::ghost_button_style
};

impl AudioData {
    #[must_use]
    pub fn sinks_submenu(
        &self,
        id: Id,
        show_more: bool,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        audio_submenu(
            icons,
            self.sinks
                .iter()
                .flat_map(|s| {
                    s.ports.iter().map(|p| SubmenuEntry {
                        name:   format!("{}: {}", p.description, s.description),
                        device: p.device_type,
                        active: p.active && s.name == self.server_info.default_sink,
                        msg:    Message::Audio(AudioMessage::DefaultSinkChanged(
                            s.name.clone(),
                            p.name.clone()
                        ))
                    })
                })
                .collect(),
            if show_more {
                Some(Message::Audio(AudioMessage::SinksMore(id)))
            } else {
                None
            },
            opacity
        )
    }

    #[must_use]
    pub fn sources_submenu(
        &self,
        id: Id,
        show_more: bool,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        audio_submenu(
            icons,
            self.sources
                .iter()
                .flat_map(|s| {
                    s.ports.iter().map(|p| SubmenuEntry {
                        name:   format!("{}: {}", p.description, s.description),
                        device: p.device_type,
                        active: p.active && s.name == self.server_info.default_source,
                        msg:    Message::Audio(AudioMessage::DefaultSourceChanged(
                            s.name.clone(),
                            p.name.clone()
                        ))
                    })
                })
                .collect(),
            if show_more {
                Some(Message::Audio(AudioMessage::SourcesMore(id)))
            } else {
                None
            },
            opacity
        )
    }
}

#[derive(Debug)]
pub struct SubmenuEntry<Message> {
    pub name:   String,
    pub device: DeviceType,
    pub active: bool,
    pub msg:    Message
}

pub fn audio_submenu<'a, Message: 'a + Clone>(
    icons: &IconTheme,
    entries: Vec<SubmenuEntry<Message>>,
    more_msg: Option<Message>,
    opacity: f32
) -> Element<'a, Message> {
    let entries = Column::with_children(
        entries
            .into_iter()
            .map(|e| {
                if e.active {
                    container(
                        row!(icon(icons, e.device.get_icon()), text(e.name))
                            .align_y(Alignment::Center)
                            .spacing(scale::scaled(16.0))
                            .padding([scale::scaled(4.0), scale::scaled(12.0)])
                    )
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.palette().success),
                        ..Default::default()
                    })
                    .into()
                } else {
                    button(
                        row!(icon(icons, e.device.get_icon()), text(e.name))
                            .spacing(scale::scaled(16.0))
                            .align_y(Alignment::Center)
                    )
                    .on_press(e.msg)
                    .padding([scale::scaled(4.0), scale::scaled(12.0)])
                    .width(Length::Fill)
                    .style(ghost_button_style(opacity))
                    .into()
                }
            })
            .collect::<Vec<_>>()
    )
    .spacing(scale::scaled(4.0))
    .into();

    match more_msg {
        Some(more_msg) => column!(
            entries,
            rule::horizontal(1),
            button("More")
                .on_press(more_msg)
                .padding([scale::scaled(4.0), scale::scaled(12.0)])
                .width(Length::Fill)
                .style(ghost_button_style(opacity)),
        )
        .spacing(scale::scaled(12.0))
        .into(),
        _ => entries
    }
}
