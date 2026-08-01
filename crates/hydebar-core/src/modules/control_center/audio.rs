use iced::{
    Alignment, Element, Length, SurfaceId as Id, Theme,
    widget::{Column, Row, button, column, container, row, rule, slider}
};

use super::{Message, SubMenu};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale,
        text::text
    },
    services::{
        ServiceEvent,
        audio::{AudioData, AudioService, DeviceType, Sinks}
    },
    style::{ghost_button_style, settings_button_style}
};

#[derive(Debug, Clone)]
pub enum AudioMessage {
    Event(Box<ServiceEvent<AudioService>>),
    DefaultSinkChanged(String, String),
    DefaultSourceChanged(String, String),
    ToggleSinkMute,
    SinkVolumeChanged(i32),
    /// A wheel notch over the bar entry, `1` up and `-1` down.
    SinkVolumeWheel(i32),
    ToggleSourceMute,
    SourceVolumeChanged(i32),
    SinksMore(Id),
    SourcesMore(Id)
}

/// The wheel notch as a volume direction: `1` up, `-1` down.
///
/// Stated once next to the message it feeds, so every place that takes the
/// wheel — the bar entry, the open menu — reads the same direction.
#[must_use]
pub fn wheel_direction(delta: iced::mouse::ScrollDelta) -> i32 {
    use iced::mouse::ScrollDelta;

    let up = match delta {
        ScrollDelta::Lines {
            y, ..
        }
        | ScrollDelta::Pixels {
            y, ..
        } => y > 0.0
    };

    if up { 1 } else { -1 }
}

impl AudioData {
    pub fn sink_indicator<Message: 'static>(
        &self,
        icons: &IconTheme
    ) -> Option<Element<'static, Message>> {
        if !self.sinks.is_empty() {
            let icon_type = self.sinks.get_icon(&self.server_info.default_sink);

            Some(icon(icons, icon_type).into())
        } else {
            None
        }
    }

    pub fn audio_sliders(
        &self,
        sub_menu: Option<SubMenu>,
        opacity: f32,
        icons: &IconTheme
    ) -> (Option<Element<'_, Message>>, Option<Element<'_, Message>>) {
        let active_sink = self
            .sinks
            .iter()
            .find(|sink| sink.name == self.server_info.default_sink);

        let sink_slider = active_sink.map(|s| {
            audio_slider(
                icons,
                SliderType::Sink,
                s.is_mute,
                Message::Audio(AudioMessage::ToggleSinkMute),
                self.cur_sink_volume,
                |v| Message::Audio(AudioMessage::SinkVolumeChanged(v)),
                if self.sinks.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                    Some((sub_menu, Message::ToggleSubMenu(SubMenu::Sinks)))
                } else {
                    None
                },
                opacity
            )
        });

        if self.sources.iter().any(|source| source.in_use) {
            let active_source = self
                .sources
                .iter()
                .find(|source| source.name == self.server_info.default_source);

            let source_slider = active_source.map(|s| {
                audio_slider(
                    icons,
                    SliderType::Source,
                    s.is_mute,
                    Message::Audio(AudioMessage::ToggleSourceMute),
                    self.cur_source_volume,
                    |v| Message::Audio(AudioMessage::SourceVolumeChanged(v)),
                    if self.sources.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                        Some((sub_menu, Message::ToggleSubMenu(SubMenu::Sources)))
                    } else {
                        None
                    },
                    opacity
                )
            });

            (sink_slider, source_slider)
        } else {
            (sink_slider, None)
        }
    }

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

pub enum SliderType {
    Sink,
    Source
}

pub fn audio_slider<'a, Message: 'a + Clone>(
    icons: &IconTheme,
    slider_type: SliderType,
    is_mute: bool,
    toggle_mute: Message,
    volume: i32,
    volume_changed: impl Fn(i32) -> Message + 'a,
    with_submenu: Option<(Option<SubMenu>, Message)>,
    opacity: f32
) -> Element<'a, Message> {
    Row::new()
        .push(
            button(icon(
                icons,
                if is_mute {
                    match slider_type {
                        SliderType::Sink => Icons::Speaker0,
                        SliderType::Source => Icons::Mic0
                    }
                } else {
                    match slider_type {
                        SliderType::Sink => Icons::Speaker3,
                        SliderType::Source => Icons::Mic1
                    }
                }
            ))
            .padding([
                8,
                match slider_type {
                    SliderType::Sink => 13,
                    SliderType::Source => 14
                }
            ])
            .on_press(toggle_mute)
            .style(settings_button_style(opacity))
        )
        .push(
            slider(0..=100, volume, volume_changed)
                .step(1)
                .width(Length::Fill)
        )
        .push_maybe(with_submenu.map(|(submenu, msg)| {
            button(icon(
                icons,
                match (slider_type, submenu) {
                    (SliderType::Sink, Some(SubMenu::Sinks)) => Icons::Close,
                    (SliderType::Source, Some(SubMenu::Sources)) => Icons::Close,
                    _ => Icons::RightArrow
                }
            ))
            .padding([scale::scaled(8.0), scale::scaled(13.0)])
            .on_press(msg)
            .style(settings_button_style(opacity))
        }))
        .align_y(Alignment::Center)
        .spacing(scale::scaled(8.0))
        .into()
}

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
