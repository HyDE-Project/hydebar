//! Drawing of the media player: the bar entry and the menu behind it.

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
    menu::MenuType,
    modules::OnModulePress,
    services::mpris::PlaybackStatus,
    style::settings_button_style
};

impl MediaPlayer {
    /// The bar entry: a note and the title of the leading player, opening the
    /// full controls.
    ///
    /// Draws nothing while no player is running, so a silent session carries
    /// no entry at all.
    ///
    /// Rendered by the module itself, so the bar layer holds no player
    /// drawing of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        config: &MediaPlayerModuleConfig,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static
    {
        let leading = self.service.as_ref()?.first()?;

        let title = self
            .bar_title
            .clone()
            .unwrap_or_else(|| Self::get_title(leading, config));

        Some((
            row![
                icon(icons, Icons::MusicNote),
                text(title)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .size(scale::scaled(12.0))
            ]
            .align_y(Vertical::Center)
            .spacing(scale::scaled(8.0))
            .into(),
            Some(OnModulePress::ToggleMenu(MenuType::MediaPlayer))
        ))
    }

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
