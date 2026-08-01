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

#[derive(Default)]
pub struct MediaPlayer {
    service: Option<MprisPlayerService>,
    sender:  Option<ModuleEventSender<Message>>,
    runtime: Option<Handle>,
    tasks:   Vec<JoinHandle<()>>
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
        Box::pin(ready(
            self.sender
                .try_send(Message::Event(Box::new(event)))
                .map_err(ModuleError::from)
        ))
    }
}

mod commands {
    //! Dispatch of player commands to the service.

    use log::warn;

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
        pub(super) fn handle_command(&mut self, service_name: String, command: PlayerCommand) {
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

                    if let Err(err) = sender.try_send(Message::Event(Box::new(event))) {
                        warn!("failed to publish media player command result: {err}");
                    }
                });
            }
        }

        pub(super) fn get_title(d: &MprisPlayerData, config: &MediaPlayerModuleConfig) -> String {
            match &d.metadata {
                Some(m) => truncate_text(&m.to_string(), config.max_title_length),
                None => "No Title".to_string()
            }
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
                        text(Self::get_title(&s[0], config))
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
        pub fn update(&mut self, message: Message) {
            match message {
                Message::Prev(s) => self.handle_command(s, PlayerCommand::Prev),
                Message::PlayPause(s) => self.handle_command(s, PlayerCommand::PlayPause),
                Message::Next(s) => self.handle_command(s, PlayerCommand::Next),
                Message::SetVolume(s, v) => self.handle_command(s, PlayerCommand::Volume(v)),
                Message::Event(event) => match *event {
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
            match &self.service {
                None => text("Not connected to MPRIS service").into(),
                Some(s) => column!(
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
        }
    }
}

#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests {
    #[cfg(all(test, feature = "enable-broken-tests"))]
    mod tests {
        use std::{
            num::NonZeroUsize,
            sync::{
                Arc,
                atomic::{AtomicBool, AtomicUsize, Ordering}
            },
            time::Duration
        };

        use futures::future::pending;
        use tokio::{task::yield_now, time::timeout};

        use super::*;
        use crate::{
            event_bus::{BusEvent, EventBus, ModuleEvent as BusModuleEvent},
            services::mpris::test_support::{
                ExecuteCommandCallback, StartListeningCallback, install_execute_command_override,
                install_start_listening_override
            }
        };

        async fn recv_event(receiver: &mut crate::event_bus::EventReceiver) -> BusEvent {
            loop {
                if let Some(event) = receiver
                    .try_recv()
                    .expect("event bus receiver should not be poisoned")
                {
                    return event;
                }

                yield_now().await;
            }
        }

        struct CancellationProbe {
            flag: Arc<AtomicBool>
        }

        impl Drop for CancellationProbe {
            fn drop(&mut self) {
                self.flag.store(true, Ordering::SeqCst);
            }
        }

        #[tokio::test]
        async fn command_success_emits_refresh_event() {
            let listener_callback: StartListeningCallback = Arc::new(|state, _publisher| {
                let _ = state;
                Box::pin(async { pending::<Result<ListenerState, ModuleError>>().await })
            });
            let _listener_guard = install_start_listening_override(listener_callback);

            let command_callback: ExecuteCommandCallback =
                Arc::new(|_service, _command| Box::pin(async { Ok(Vec::new()) }));
            let _command_guard = install_execute_command_override(command_callback);

            let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
            let mut receiver = bus.receiver();
            let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

            let mut media_player = MediaPlayer::default();
            assert!(
                <MediaPlayer as Module<Message>>::register(&mut media_player, &context, ())
                    .is_ok()
            );

            media_player.handle_command("player".to_string(), PlayerCommand::Next);

            let event = timeout(Duration::from_secs(1), recv_event(&mut receiver))
                .await
                .expect("media player event should be emitted");

            match event {
                BusEvent::Module(BusModuleEvent::MediaPlayer(Message::Event(
                    ServiceEvent::Update(MprisPlayerEvent::Refresh(data))
                ))) => {
                    assert!(data.is_empty());
                }
                other => panic!("unexpected event: {other:?}")
            }

            for task in media_player.tasks.drain(..) {
                task.abort();
            }
        }

        #[tokio::test]
        #[ignore = "Timing-sensitive test - needs rework"]
        async fn command_failure_emits_error_event() {
            let listener_callback: StartListeningCallback = Arc::new(|state, _publisher| {
                let _ = state;
                Box::pin(async { pending::<Result<ListenerState, ModuleError>>().await })
            });
            let _listener_guard = install_start_listening_override(listener_callback);

            let error = ModuleError::registration("command failure");
            let command_callback: ExecuteCommandCallback = Arc::new({
                let error = error.clone();
                move |_service, _command| {
                    let error = error.clone();
                    Box::pin(async move { Err(error) })
                }
            });
            let _command_guard = install_execute_command_override(command_callback);

            let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
            let mut receiver = bus.receiver();
            let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

            let mut media_player = MediaPlayer::default();
            assert!(
                <MediaPlayer as Module<Message>>::register(&mut media_player, &context, ())
                    .is_ok()
            );

            media_player.handle_command("player".to_string(), PlayerCommand::PlayPause);

            let event = timeout(Duration::from_secs(1), recv_event(&mut receiver))
                .await
                .expect("media player event should be emitted");

            match event {
                BusEvent::Module(BusModuleEvent::MediaPlayer(Message::Event(
                    ServiceEvent::Error(err)
                ))) => {
                    assert_eq!(err, error);
                }
                other => panic!("unexpected event: {other:?}")
            }

            for task in media_player.tasks.drain(..) {
                task.abort();
            }
        }

        #[tokio::test]
        #[ignore = "Timing-sensitive test - needs rework"]
        async fn register_aborts_previous_listener() {
            let cancelled = Arc::new(AtomicBool::new(false));
            let call_count = Arc::new(AtomicUsize::new(0));

            let listener_callback: StartListeningCallback = Arc::new({
                let cancelled = Arc::clone(&cancelled);
                let call_count = Arc::clone(&call_count);

                move |state, _publisher| {
                    call_count.fetch_add(1, Ordering::SeqCst);
                    let flag = Arc::clone(&cancelled);

                    Box::pin(async move {
                        let _probe = CancellationProbe {
                            flag
                        };
                        let _ = state;
                        pending::<Result<ListenerState, ModuleError>>().await
                    })
                }
            });
            let _listener_guard = install_start_listening_override(listener_callback);

            let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
            let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

            let mut media_player = MediaPlayer::default();
            assert!(
                <MediaPlayer as Module<Message>>::register(&mut media_player, &context, ())
                    .is_ok()
            );
            assert_eq!(call_count.load(Ordering::SeqCst), 1);

            assert!(
                <MediaPlayer as Module<Message>>::register(&mut media_player, &context, ())
                    .is_ok()
            );
            assert_eq!(call_count.load(Ordering::SeqCst), 2);

            timeout(Duration::from_secs(1), async {
                loop {
                    if cancelled.load(Ordering::SeqCst) {
                        break;
                    }
                    yield_now().await;
                }
            })
            .await
            .expect("previous listener should be cancelled");

            for task in media_player.tasks.drain(..) {
                task.abort();
            }
        }
    }
}

pub use messages::Message;
