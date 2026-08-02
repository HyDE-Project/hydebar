//! Volume slider rows of the sink and the source, with their mute
//! buttons and submenu toggles.

use iced::{
    Alignment, Element, Length,
    widget::{Row, button, slider}
};

use super::{
    super::{Message, SubMenu},
    AudioMessage
};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale
    },
    services::audio::AudioData,
    style::settings_button_style
};

impl AudioData {
    #[must_use]
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
}

#[derive(Debug)]
pub enum SliderType {
    Sink,
    Source
}

#[expect(
    clippy::too_many_arguments,
    reason = "each argument feeds a distinct visual piece of the slider row"
)]
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
                    (SliderType::Sink, Some(SubMenu::Sinks))
                    | (SliderType::Source, Some(SubMenu::Sources)) => Icons::Close,
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
