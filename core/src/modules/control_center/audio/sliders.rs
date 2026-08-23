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
    /// The output and input sliders, drawn together.
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

/// Which end of the sound a slider drives.
#[derive(Debug)]
pub enum SliderType {
    /// Something the machine plays to.
    Sink,
    /// Something the machine records from.
    Source
}

#[expect(
    clippy::too_many_arguments,
    reason = "each argument feeds a distinct visual piece of the slider row"
)]
/// One volume slider, with its mute button and its submenu arrow.
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use iced_test::simulator;
    use libpulse_binding::volume::ChannelVolumes;

    use super::*;
    use crate::services::audio::{Device, DeviceType, Port, ServerInfo};

    fn port(name: &str, active: bool) -> Port {
        Port {
            name: name.to_owned(),
            description: format!("{name} port"),
            device_type: DeviceType::Speaker,
            active
        }
    }

    fn device(name: &str, ports: Vec<Port>, is_mute: bool, in_use: bool) -> Device {
        Device {
            name: name.to_owned(),
            description: format!("{name} device"),
            volume: ChannelVolumes::default(),
            is_mute,
            in_use,
            ports
        }
    }

    fn data(sinks: Vec<Device>, sources: Vec<Device>) -> AudioData {
        AudioData {
            server_info: ServerInfo {
                default_sink:   "sink".to_owned(),
                default_source: "source".to_owned()
            },
            sinks,
            sources,
            cur_sink_volume: 40,
            cur_source_volume: 60
        }
    }

    fn icons() -> IconTheme {
        IconTheme::default()
    }

    #[test]
    fn a_desktop_without_a_default_sink_offers_no_slider() {
        let data = data(
            vec![device("other", vec![port("a", true)], false, true)],
            vec![]
        );
        let (sink, source) = data.audio_sliders(None, 1.0, &icons());

        assert!(sink.is_none());
        assert!(source.is_none());
    }

    #[test]
    fn the_default_sink_gets_a_slider() {
        let data = data(
            vec![device("sink", vec![port("a", true)], false, true)],
            vec![]
        );
        let (sink, source) = data.audio_sliders(None, 1.0, &icons());

        assert!(sink.is_some());
        assert!(source.is_none());

        let mut ui = simulator(sink.expect("the default sink has a slider"));
        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    /// Clicks the far right of the row, where the submenu toggle sits when
    /// there is one, and reports what the row published.
    fn press_the_right_edge(slider: Element<'_, Message>) -> Vec<Message> {
        let mut ui = simulator(slider);

        ui.point_at(iced::Point::new(1018.0, 12.0));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    #[test]
    fn a_sink_with_one_port_offers_no_way_into_a_submenu() {
        let data = data(
            vec![device("sink", vec![port("a", true)], false, true)],
            vec![]
        );
        let (sink, _) = data.audio_sliders(None, 1.0, &icons());

        let published = press_the_right_edge(sink.expect("the default sink has a slider"));

        assert!(
            !published
                .iter()
                .any(|message| matches!(message, Message::ToggleSubMenu(SubMenu::Sinks))),
            "one port is not a choice worth a submenu"
        );
    }

    #[test]
    fn a_sink_with_several_ports_offers_a_way_into_its_submenu() {
        let data = data(
            vec![device(
                "sink",
                vec![port("a", true), port("b", false)],
                false,
                true
            )],
            vec![]
        );
        let (sink, _) = data.audio_sliders(None, 1.0, &icons());

        let published = press_the_right_edge(sink.expect("the default sink has a slider"));

        assert!(
            published
                .iter()
                .any(|message| matches!(message, Message::ToggleSubMenu(SubMenu::Sinks)))
        );
    }

    #[test]
    fn a_source_with_several_ports_offers_a_way_into_its_submenu() {
        let data = data(
            vec![device("sink", vec![port("a", true)], false, true)],
            vec![device(
                "source",
                vec![port("a", true), port("b", false)],
                false,
                true
            )]
        );
        let (_, source) = data.audio_sliders(None, 1.0, &icons());

        let published = press_the_right_edge(source.expect("the default source has a slider"));

        assert!(
            published
                .iter()
                .any(|message| matches!(message, Message::ToggleSubMenu(SubMenu::Sources)))
        );
    }

    #[test]
    fn a_source_in_use_gets_a_slider_of_its_own() {
        let data = data(
            vec![device("sink", vec![port("a", true)], false, true)],
            vec![device("source", vec![port("a", true)], false, true)]
        );

        let (sink, source) = data.audio_sliders(None, 1.0, &icons());

        assert!(sink.is_some());
        assert!(source.is_some());
    }

    #[test]
    fn a_source_nobody_is_using_stays_out_of_the_menu() {
        let data = data(
            vec![device("sink", vec![port("a", true)], false, true)],
            vec![device("source", vec![port("a", true)], false, false)]
        );

        let (_, source) = data.audio_sliders(None, 1.0, &icons());

        assert!(source.is_none());
    }

    #[test]
    fn a_source_in_use_that_is_not_the_default_one_gets_no_slider() {
        let data = data(
            vec![device("sink", vec![port("a", true)], false, true)],
            vec![device("other", vec![port("a", true)], false, true)]
        );

        let (_, source) = data.audio_sliders(None, 1.0, &icons());

        assert!(source.is_none());
    }

    #[test]
    fn pressing_the_glyph_asks_to_mute_the_sink() {
        let data = data(
            vec![device("sink", vec![port("a", true)], false, true)],
            vec![]
        );
        let (sink, _) = data.audio_sliders(None, 1.0, &icons());

        let mut ui = simulator(sink.expect("the default sink has a slider"));
        ui.point_at(iced::Point::new(10.0, 12.0));
        let _ = ui.simulate(iced_test::simulator::click());

        assert!(
            ui.into_messages()
                .any(|message| matches!(message, Message::Audio(AudioMessage::ToggleSinkMute)))
        );
    }

    #[test]
    fn a_muted_and_an_open_slider_are_both_drawn() {
        for is_mute in [true, false] {
            for slider_type in [SliderType::Sink, SliderType::Source] {
                let element: Element<'_, Message> = audio_slider(
                    &icons(),
                    slider_type,
                    is_mute,
                    Message::Audio(AudioMessage::ToggleSinkMute),
                    50,
                    |volume| Message::Audio(AudioMessage::SinkVolumeChanged(volume)),
                    None,
                    1.0
                );

                let mut ui = simulator(element);
                assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
            }
        }
    }

    #[test]
    fn an_open_submenu_offers_to_close_itself() {
        for (slider_type, open) in [
            (SliderType::Sink, SubMenu::Sinks),
            (SliderType::Source, SubMenu::Sources)
        ] {
            let element: Element<'_, Message> = audio_slider(
                &icons(),
                slider_type,
                false,
                Message::Audio(AudioMessage::ToggleSinkMute),
                50,
                |volume| Message::Audio(AudioMessage::SinkVolumeChanged(volume)),
                Some((Some(open), Message::ToggleSubMenu(open))),
                1.0
            );

            let mut ui = simulator(element);
            assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
        }
    }

    #[test]
    fn a_closed_submenu_offers_to_open_itself() {
        let element: Element<'_, Message> = audio_slider(
            &icons(),
            SliderType::Sink,
            false,
            Message::Audio(AudioMessage::ToggleSinkMute),
            50,
            |volume| Message::Audio(AudioMessage::SinkVolumeChanged(volume)),
            Some((Some(SubMenu::Wifi), Message::ToggleSubMenu(SubMenu::Sinks))),
            1.0
        );

        let mut ui = simulator(element);
        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }
}
