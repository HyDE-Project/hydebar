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
                            warn!("failed to publish media player listener error: {send_error}");
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
    /// Every property change of every running player crosses D-Bus into this
    /// task, so leaving it connected keeps the bar repainting to the beat of a
    /// track it does not display.
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
