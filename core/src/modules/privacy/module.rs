//! Module trait wiring for the privacy indicator.
//!
//! Registration spawns the listener loop that keeps the privacy service
//! connected, backing off after failures; the view draws one icon per active
//! access and stays empty while nothing is watched.

use iced::{
    Alignment, Element,
    widget::{Row, container}
};

use super::{
    Privacy, PrivacyMessage,
    publisher::{ModulePublisher, run_start_listening}
};
use crate::{
    ModuleContext,
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale
    },
    event_bus::ModuleEvent,
    modules::{Module, ModuleError, OnModulePress},
    services::{ServiceEvent, privacy::State}
};

impl<M> Module<M> for Privacy
where
    M: 'static + Clone
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        for task in self.tasks.drain(..) {
            task.abort();
        }

        let sender = ctx.module_sender(ModuleEvent::Privacy);
        let mut publisher = ModulePublisher::new(sender.clone());
        let error_sender = sender.clone();

        let task = ctx.runtime_handle().spawn(async move {
            let mut state = State::Init;
            let mut failures: u32 = 0;

            loop {
                match run_start_listening(state, &mut publisher).await {
                    Ok(next_state) => {
                        failures = 0;
                        state = next_state;
                    }
                    Err(error) => {
                        error_sender
                            .send(PrivacyMessage::Event(ServiceEvent::Error(error.clone())));

                        failures = failures.saturating_add(1);
                        tokio::time::sleep(crate::services::reconnect_delay(failures)).await;
                        state = State::Init;
                    }
                }
            }
        });

        self.sender = Some(sender);
        self.tasks.push(task);

        Ok(())
    }

    /// Stops watching `PipeWire` and the webcam nodes once the indicator leaves
    /// the bar.
    ///
    /// The listener keeps a `PipeWire` connection and an inotify watch alive; a
    /// layout that shows no privacy dot has no use for either.
    fn deregister(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
        }

        self.sender = None;
    }

    /// Render the privacy indicator when data is available.
    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        self.service.as_ref().and_then(|service| {
            if service.no_access() {
                None
            } else {
                Some((
                    container(
                        Row::new()
                            .push_maybe(
                                service
                                    .screenshare_access()
                                    .then(|| icon(icons, Icons::ScreenShare))
                            )
                            .push_maybe(
                                service.webcam_access().then(|| icon(icons, Icons::Webcam))
                            )
                            .push_maybe(
                                service
                                    .microphone_access()
                                    .then(|| icon(icons, Icons::Mic1))
                            )
                            .align_y(Alignment::Center)
                            .spacing(scale::item_gap())
                    )
                    .style(|theme| container::Style {
                        text_color: Some(theme.extended_palette().danger.weak.color),
                        ..Default::default()
                    })
                    .into(),
                    None
                ))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::event_bus::EventBus;

    fn context(bus: &EventBus) -> ModuleContext {
        ModuleContext::new(bus.sender(), tokio::runtime::Handle::current())
    }

    #[tokio::test]
    async fn re_registration_replaces_the_listener_instead_of_stacking_one() {
        let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
        let ctx = context(&bus);
        let mut privacy = Privacy::default();

        <Privacy as Module<()>>::register(&mut privacy, &ctx, ()).expect("first registration");
        <Privacy as Module<()>>::register(&mut privacy, &ctx, ()).expect("second registration");

        assert_eq!(
            privacy.tasks.len(),
            1,
            "the first listener must be aborted, not accumulated"
        );

        <Privacy as Module<()>>::deregister(&mut privacy);
    }

    #[tokio::test]
    async fn deregistration_releases_the_listener_and_the_sender() {
        let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
        let ctx = context(&bus);
        let mut privacy = Privacy::default();

        <Privacy as Module<()>>::register(&mut privacy, &ctx, ()).expect("registration");
        <Privacy as Module<()>>::deregister(&mut privacy);

        assert!(privacy.tasks.is_empty(), "the listener task must be gone");
        assert!(privacy.sender.is_none(), "the sender must be released");
    }
}
