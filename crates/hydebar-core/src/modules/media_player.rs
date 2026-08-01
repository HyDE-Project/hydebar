use std::{
    future::{Future, ready},
    pin::Pin
};

use tokio::{runtime::Handle, task::JoinHandle};

use super::ModuleError;
use crate::{
    ModuleEventSender,
    services::{
        ServiceEvent,
        mpris::{MprisEventPublisher, MprisPlayerService}
    }
};

#[derive(Debug, Default)]
pub struct MediaPlayer {
    service:   Option<MprisPlayerService>,
    sender:    Option<ModuleEventSender<Message>>,
    runtime:   Option<Handle>,
    tasks:     Vec<JoinHandle<()>>,
    /// The bar line for the leading player, rendered when the state moves.
    ///
    /// Composed once per service event instead of per frame: the join,
    /// the format and the truncation ran on every repaint for a value
    /// that only changes on a track or player change.
    bar_title: Option<String>
}

struct MediaPlayerPublisher {
    sender: ModuleEventSender<Message>
}

impl MediaPlayerPublisher {
    const fn new(sender: ModuleEventSender<Message>) -> Self {
        Self {
            sender
        }
    }
}

impl MprisEventPublisher for MediaPlayerPublisher {
    fn send(
        &mut self,
        event: ServiceEvent<MprisPlayerService>
    ) -> Pin<Box<dyn Future<Output = Result<(), ModuleError>> + Send + '_>> {
        self.sender.send(Message::Event(Box::new(event)));

        Box::pin(ready(Ok(())))
    }
}

mod commands {
    //! Dispatch of player commands to the service.

    use super::{MediaPlayer, Message};
    use crate::{
        config::MediaPlayerModuleConfig,
        services::{
            ServiceEvent,
            mpris::{
                MprisPlayerCommand, MprisPlayerData, MprisPlayerEvent, MprisPlayerService,
                PlayerCommand
            }
        },
        utils::truncate_text
    };

    impl MediaPlayer {
        pub(super) fn handle_command(&self, service_name: String, command: PlayerCommand) {
            let runtime = self.runtime.clone();
            let sender = self.sender.clone();
            let service = self.service.clone();

            if let (Some(runtime), Some(sender)) = (runtime, sender) {
                runtime.spawn(async move {
                    let result = MprisPlayerService::execute_command(
                        service,
                        MprisPlayerCommand {
                            service_name,
                            command
                        }
                    )
                    .await;

                    let event = match result {
                        Ok(data) => ServiceEvent::Update(MprisPlayerEvent::Refresh(data)),
                        Err(error) => ServiceEvent::Error(error)
                    };

                    sender.send(Message::Event(Box::new(event)));
                });
            }
        }

        pub(super) fn get_title(d: &MprisPlayerData, config: &MediaPlayerModuleConfig) -> String {
            d.metadata.as_ref().map_or_else(
                || "No Title".to_string(),
                |m| truncate_text(&m.to_string(), config.max_title_length)
            )
        }
    }
}
mod messages {
    //! Messages accepted by the media player module.

    use crate::services::{ServiceEvent, mpris::MprisPlayerService};

    #[derive(Debug, Clone)]
    pub enum Message {
        Prev(String),
        PlayPause(String),
        Next(String),
        SetVolume(String, f64),
        Event(Box<ServiceEvent<MprisPlayerService>>)
    }
}
mod module {
    //! Module trait wiring for the media player.

    use iced::{Element, alignment::Vertical, widget::row};
    use log::warn;

    use super::{MediaPlayer, MediaPlayerPublisher};
    use crate::{
        ModuleContext,
        components::{
            icons::{IconTheme, Icons, icon},
            scale,
            text::text
        },
        config::MediaPlayerModuleConfig,
        event_bus::ModuleEvent,
        menu::MenuType,
        modules::{Module, ModuleError, OnModulePress},
        services::{
            ServiceEvent,
            mpris::{ListenerState, MprisEventPublisher, MprisPlayerService}
        }
    };

    impl<M> Module<M> for MediaPlayer
    where
        M: 'static + Clone
    {
        type ViewData<'a> = (&'a MediaPlayerModuleConfig, &'a IconTheme);
        type RegistrationData<'a> = ();

        fn register(
            &mut self,
            ctx: &ModuleContext,
            (): Self::RegistrationData<'_>
        ) -> Result<(), ModuleError> {
            for task in self.tasks.drain(..) {
                task.abort();
            }

            self.service = None;

            let sender = ctx.module_sender(ModuleEvent::MediaPlayer);
            let listener_sender = sender.clone();

            let task = ctx.runtime_handle().spawn(async move {
                let mut state = ListenerState::Init;
                let mut publisher = MediaPlayerPublisher::new(listener_sender);
                let mut failures: u32 = 0;

                loop {
                    match MprisPlayerService::start_listening(state, &mut publisher).await {
                        Ok(next_state) => {
                            failures = 0;
                            state = next_state;
                        }
                        Err(error) => {
                            let publish_result =
                                publisher.send(ServiceEvent::Error(error.clone())).await;

                            if let Err(send_error) = publish_result {
                                warn!(
                                    "failed to publish media player listener error: {send_error}"
                                );
                                break;
                            }

                            failures = failures.saturating_add(1);
                            tokio::time::sleep(crate::services::reconnect_delay(failures)).await;
                            state = ListenerState::Init;
                        }
                    }
                }
            });

            self.sender = Some(sender);
            self.runtime = Some(ctx.runtime_handle().clone());
            self.tasks.push(task);

            Ok(())
        }

        /// Drops the MPRIS listener once the player leaves the bar.
        ///
        /// Every property change of every running player crosses D-Bus into
        /// this task, so leaving it connected keeps the bar repainting
        /// to the beat of a track it does not display.
        fn deregister(&mut self) {
            for task in self.tasks.drain(..) {
                task.abort();
            }

            self.service = None;
            self.sender = None;
        }

        fn view(
            &self,
            (config, icons): Self::ViewData<'_>
        ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
            self.service.as_ref().and_then(|s| match s.len() {
                0 => None,
                _ => Some((
                    row![
                        icon(icons, Icons::MusicNote),
                        text(
                            self.bar_title
                                .clone()
                                .unwrap_or_else(|| Self::get_title(&s[0], config))
                        )
                        .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                        .size(scale::scaled(12.0))
                    ]
                    .align_y(Vertical::Center)
                    .spacing(scale::scaled(8.0))
                    .into(),
                    Some(OnModulePress::ToggleMenu(MenuType::MediaPlayer))
                ))
            })
        }
    }
}
mod state {
    //! Message handling for the media player module.

    use log::error;

    use super::{MediaPlayer, Message};
    use crate::services::{ReadOnlyService, ServiceEvent, mpris::PlayerCommand};

    impl MediaPlayer {
        pub fn update(
            &mut self,
            message: Message,
            config: &hydebar_proto::config::MediaPlayerModuleConfig
        ) {
            match message {
                Message::Prev(s) => self.handle_command(s, PlayerCommand::Prev),
                Message::PlayPause(s) => self.handle_command(s, PlayerCommand::PlayPause),
                Message::Next(s) => self.handle_command(s, PlayerCommand::Next),
                Message::SetVolume(s, v) => self.handle_command(s, PlayerCommand::Volume(v)),
                Message::Event(event) => {
                    match *event {
                        ServiceEvent::Init(s) => {
                            self.service = Some(s);
                        }
                        ServiceEvent::Update(d) => {
                            if let Some(service) = self.service.as_mut() {
                                service.update(d);
                            }
                        }
                        ServiceEvent::Error(error) => {
                            error!("media player service error: {error}");
                        }
                    }

                    self.bar_title = self
                        .service
                        .as_ref()
                        .and_then(|s| s.first())
                        .map(|d| Self::get_title(d, config));
                }
            }
        }
    }
}
mod view {
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
}

pub use messages::Message;
