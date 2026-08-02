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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use iced_test::simulator;
    use libpulse_binding::volume::ChannelVolumes;

    use super::*;
    use crate::services::audio::{Device, Port, ServerInfo};

    /// The surface a submenu opens more entries into, which the test never
    /// looks at beyond passing it along.
    fn surface() -> Id {
        Id::unique()
    }

    fn port(name: &str, active: bool) -> Port {
        Port {
            name: name.to_owned(),
            description: format!("{name} out"),
            device_type: DeviceType::Speaker,
            active
        }
    }

    fn device(name: &str, ports: Vec<Port>) -> Device {
        Device {
            name: name.to_owned(),
            description: format!("{name} card"),
            volume: ChannelVolumes::default(),
            is_mute: false,
            in_use: true,
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
    fn every_port_of_every_sink_is_listed_by_card_and_port() {
        let data = data(
            vec![
                device("sink", vec![port("speakers", true)]),
                device("other", vec![port("hdmi", false)]),
            ],
            vec![]
        );

        let mut ui = simulator(data.sinks_submenu(surface(), false, 1.0, &icons()));

        assert!(ui.find("speakers out: sink card").is_ok());
        assert!(ui.find("hdmi out: other card").is_ok());
        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }

    #[test]
    fn the_port_in_use_on_the_default_sink_answers_no_press() {
        let data = data(
            vec![device(
                "sink",
                vec![port("speakers", true), port("hdmi", false)]
            )],
            vec![]
        );

        let mut ui = simulator(data.sinks_submenu(surface(), false, 1.0, &icons()));
        let _ = ui
            .click("speakers out: sink card")
            .expect("the active entry is drawn");

        assert!(
            ui.into_messages().next().is_none(),
            "switching to the port already in use is not a deed"
        );
    }

    #[test]
    fn picking_another_port_asks_for_that_sink_and_port() {
        let data = data(
            vec![device(
                "sink",
                vec![port("speakers", true), port("hdmi", false)]
            )],
            vec![]
        );

        let mut ui = simulator(data.sinks_submenu(surface(), false, 1.0, &icons()));
        let _ = ui
            .click("hdmi out: sink card")
            .expect("the idle entry is pressable");

        assert!(ui.into_messages().any(|message| matches!(
            message,
            Message::Audio(AudioMessage::DefaultSinkChanged(card, chosen))
                if card == "sink" && chosen == "hdmi"
        )));
    }

    #[test]
    fn a_port_active_on_a_card_that_is_not_the_default_one_stays_pressable() {
        let data = data(vec![device("other", vec![port("hdmi", true)])], vec![]);

        let mut ui = simulator(data.sinks_submenu(surface(), false, 1.0, &icons()));
        let _ = ui
            .click("hdmi out: other card")
            .expect("the entry is pressable");

        assert!(
            ui.into_messages().any(|message| matches!(
                message,
                Message::Audio(AudioMessage::DefaultSinkChanged(card, chosen))
                    if card == "other" && chosen == "hdmi"
            )),
            "a port lit on another card is still somewhere to switch to"
        );
    }

    #[test]
    fn every_port_of_every_source_is_listed() {
        let data = data(
            vec![],
            vec![device(
                "source",
                vec![port("mic", true), port("line", false)]
            )]
        );

        let mut ui = simulator(data.sources_submenu(surface(), false, 1.0, &icons()));

        assert!(ui.find("mic out: source card").is_ok());
        assert!(ui.find("line out: source card").is_ok());
    }

    #[test]
    fn picking_another_source_port_asks_for_that_source_and_port() {
        let data = data(
            vec![],
            vec![device(
                "source",
                vec![port("mic", true), port("line", false)]
            )]
        );

        let mut ui = simulator(data.sources_submenu(surface(), false, 1.0, &icons()));
        let _ = ui
            .click("line out: source card")
            .expect("the idle entry is pressable");

        assert!(ui.into_messages().any(|message| matches!(
            message,
            Message::Audio(AudioMessage::DefaultSourceChanged(card, chosen))
                if card == "source" && chosen == "line"
        )));
    }

    #[test]
    fn a_submenu_that_can_show_more_carries_the_offer() {
        let data = data(vec![device("sink", vec![port("speakers", true)])], vec![]);

        let mut ui = simulator(data.sinks_submenu(surface(), true, 1.0, &icons()));

        assert!(ui.find("More").is_ok());
    }

    #[test]
    fn a_submenu_that_holds_everything_offers_nothing_more() {
        let data = data(vec![device("sink", vec![port("speakers", true)])], vec![]);

        let mut ui = simulator(data.sinks_submenu(surface(), false, 1.0, &icons()));

        assert!(ui.find("More").is_err());
    }

    #[test]
    fn pressing_more_asks_the_sink_list_to_open_wider() {
        let data = data(vec![device("sink", vec![port("speakers", true)])], vec![]);

        let mut ui = simulator(data.sinks_submenu(surface(), true, 1.0, &icons()));
        let _ = ui.click("More").expect("the offer is pressable");

        assert!(
            ui.into_messages()
                .any(|message| matches!(message, Message::Audio(AudioMessage::SinksMore(_))))
        );
    }

    #[test]
    fn pressing_more_asks_the_source_list_to_open_wider() {
        let data = data(vec![], vec![device("source", vec![port("mic", true)])]);

        let mut ui = simulator(data.sources_submenu(surface(), true, 1.0, &icons()));
        let _ = ui.click("More").expect("the offer is pressable");

        assert!(
            ui.into_messages()
                .any(|message| matches!(message, Message::Audio(AudioMessage::SourcesMore(_))))
        );
    }

    #[test]
    fn a_desktop_with_no_devices_still_draws_its_submenu() {
        let data = data(vec![], vec![]);

        let mut ui = simulator(data.sinks_submenu(surface(), false, 1.0, &icons()));

        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }
}
