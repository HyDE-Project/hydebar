//! Bar entry and menu of the standalone audio module.

use iced::{
    Element, Length, SurfaceId as Id,
    widget::{Column, mouse_area}
};

use super::super::helpers::sub_menu_wrapper;
use crate::{
    components::{icons::IconTheme, push_maybe::PushMaybe, scale},
    config::{ControlCenterModuleConfig, Position},
    menu::MenuType,
    modules::{
        OnModulePress,
        control_center::{
            audio::{AudioMessage, wheel_direction},
            state::{ControlCenter, Message, SubMenu}
        }
    }
};

impl ControlCenter {
    /// Bar entry of the standalone audio module.
    ///
    /// Renders nothing while the audio service is away, so a session
    /// without a sound server keeps a bar free of dead
    /// icons.
    ///
    /// The entry answers the wheel as well: a notch up or down nudges
    /// the sink volume without the menu ever opening, the
    /// way the reference waybar module behaves.
    #[must_use]
    pub fn audio_bar<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message> + Clone
    {
        let indicator = self.audio.as_ref().and_then(|a| a.sink_indicator(icons))?;
        let wheeled = mouse_area(indicator)
            .on_scroll(|delta| {
                M::from(Message::Audio(AudioMessage::SinkVolumeWheel(
                    wheel_direction(delta)
                )))
            })
            .into();

        Some((wheeled, Some(OnModulePress::ToggleMenu(MenuType::Audio))))
    }

    /// Menu of the standalone audio module: both sliders and their
    /// device lists.
    #[must_use]
    pub fn audio_menu(
        &self,
        id: Id,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        position: Position,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        let (sink_slider, source_slider) = self.audio.as_ref().map_or((None, None), |a| {
            a.audio_sliders(self.sub_menu, opacity, icons)
        });

        let (top_sink_slider, bottom_sink_slider) = match position {
            Position::Top => (sink_slider, None),
            Position::Bottom => (None, sink_slider)
        };
        let (top_source_slider, bottom_source_slider) = match position {
            Position::Top => (source_slider, None),
            Position::Bottom => (None, source_slider)
        };

        Column::new()
            .push_maybe(top_sink_slider)
            .push_maybe(
                self.sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Sinks)
                    .and_then(|_| {
                        self.audio.as_ref().map(|a| {
                            sub_menu_wrapper(
                                a.sinks_submenu(
                                    id,
                                    config.audio_sinks_more_cmd.is_some(),
                                    opacity,
                                    icons
                                ),
                                opacity
                            )
                        })
                    })
            )
            .push_maybe(bottom_sink_slider)
            .push_maybe(top_source_slider)
            .push_maybe(
                self.sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Sources)
                    .and_then(|_| {
                        self.audio.as_ref().map(|a| {
                            sub_menu_wrapper(
                                a.sources_submenu(
                                    id,
                                    config.audio_sources_more_cmd.is_some(),
                                    opacity,
                                    icons
                                ),
                                opacity
                            )
                        })
                    })
            )
            .push_maybe(bottom_source_slider)
            .width(Length::Fill)
            .spacing(scale::scaled(16.0))
            .into()
    }
}
