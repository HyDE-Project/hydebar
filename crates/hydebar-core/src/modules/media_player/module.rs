//! Module trait wiring for the media player.

use iced::{
    Element,
    alignment::Vertical,
    widget::{row, text}
};
use log::warn;
use tokio::task::yield_now;

use super::{MediaPlayer, MediaPlayerPublisher};
use crate::{
    ModuleContext,
    components::icons::{Icons, icon},
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
    type ViewData<'a> = &'a MediaPlayerModuleConfig;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_>
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

            loop {
                match MprisPlayerService::start_listening(state, &mut publisher).await {
                    Ok(next_state) => {
                        state = next_state;
                    }
                    Err(error) => {
                        let publish_result =
                            publisher.send(ServiceEvent::Error(error.clone())).await;

                        if let Err(send_error) = publish_result {
                            warn!("failed to publish media player listener error: {send_error}");
                            break;
                        }

                        state = ListenerState::Init;
                        yield_now().await;
                    }
                }
            }
        });

        self.sender = Some(sender);
        self.runtime = Some(ctx.runtime_handle().clone());
        self.tasks.push(task);

        Ok(())
    }

    fn view(
        &self,
        config: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        self.service.as_ref().and_then(|s| match s.len() {
            0 => None,
            _ => Some((
                row![
                    icon(Icons::MusicNote),
                    text(Self::get_title(&s[0], config))
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .size(12)
                ]
                .align_y(Vertical::Center)
                .spacing(8)
                .into(),
                Some(OnModulePress::ToggleMenu(MenuType::MediaPlayer))
            ))
        })
    }
}
