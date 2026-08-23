//! Rendering of the media player menu.

use iced::{
    Background, Border, Element, Length, Theme,
    alignment::Vertical,
    widget::{Column, button, column, container, row, rule, slider}
};

use super::{MediaPlayer, Message};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale,
        text::text
    },
    config::MediaPlayerModuleConfig,
    services::mpris::PlaybackStatus,
    style::settings_button_style
};

impl MediaPlayer {
    /// The panel naming every player and driving each.
    #[must_use]
    pub fn menu_view(
        &self,
        config: &MediaPlayerModuleConfig,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        self.service.as_ref().map_or_else(
            || text("Not connected to MPRIS service").into(),
            |s| {
                column!(
                    text("Players").size(scale::scaled(20.0)),
                    rule::horizontal(1),
                    column(s.iter().map(|d| {
                        let title = text(Self::get_title(d, config))
                            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                            .width(Length::Fill);

                        let play_pause_icon = match d.state {
                            PlaybackStatus::Playing => Icons::Pause,
                            PlaybackStatus::Paused | PlaybackStatus::Stopped => Icons::Play
                        };

                        let buttons = row![
                            button(icon(icons, Icons::SkipPrevious))
                                .on_press(Message::Prev(d.service.clone()))
                                .padding([scale::scaled(5.0), scale::scaled(12.0)])
                                .style(settings_button_style(opacity)),
                            button(icon(icons, play_pause_icon))
                                .on_press(Message::PlayPause(d.service.clone()))
                                .style(settings_button_style(opacity)),
                            button(icon(icons, Icons::SkipNext))
                                .on_press(Message::Next(d.service.clone()))
                                .padding([scale::scaled(5.0), scale::scaled(12.0)])
                                .style(settings_button_style(opacity)),
                        ]
                        .spacing(scale::scaled(8.0));

                        let volume_slider = d.volume.map(|v| {
                            slider(0.0..=100.0, v, move |v| {
                                Message::SetVolume(d.service.clone(), v)
                            })
                        });

                        container(
                            Column::new()
                                .push(
                                    row!(title, buttons)
                                        .spacing(scale::scaled(8.0))
                                        .align_y(Vertical::Center)
                                )
                                .push_maybe(volume_slider)
                                .spacing(scale::scaled(8.0))
                        )
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
                        .padding(scale::scaled(16.0))
                        .width(Length::Fill)
                        .into()
                    }))
                    .spacing(scale::scaled(16.0))
                )
                .spacing(scale::scaled(8.0))
                .into()
            }
        )
    }
}
