use std::future::{Ready, ready};

use iced::{
    Alignment, Element,
    widget::{Row, container}
};
use log::{error, warn};
use tokio::task::JoinHandle;

use super::{Module, ModuleError, OnModulePress};
#[cfg(test)]
use crate::event_bus::BusEvent;
use crate::{
    ModuleContext, ModuleEventSender,
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale
    },
    event_bus::ModuleEvent,
    services::{
        ReadOnlyService, ServiceEvent,
        privacy::{PrivacyEventPublisher, PrivacyService, State, error::PrivacyError}
    }
};

/// Message emitted by the privacy module subscription.
#[derive(Debug, Clone)]
pub enum PrivacyMessage {
    Event(ServiceEvent<PrivacyService>)
}

/// UI module exposing privacy information icons.
#[derive(Debug, Default)]
pub struct Privacy {
    pub service: Option<PrivacyService>,
    sender:      Option<ModuleEventSender<PrivacyMessage>>,
    tasks:       Vec<JoinHandle<()>>
}

impl Privacy {
    /// Update the module state based on new privacy events.
    pub fn update(&mut self, message: PrivacyMessage) {
        let PrivacyMessage::Event(event) = message;
        match event {
            ServiceEvent::Init(service) => {
                self.service = Some(service);
            }
            ServiceEvent::Update(data) => {
                if let Some(privacy) = self.service.as_mut() {
                    privacy.update(data);
                }
            }
            ServiceEvent::Error(error) => match error {
                PrivacyError::WebcamUnavailable => {
                    warn!("Webcam device unavailable; continuing with PipeWire-only privacy data");
                }
                _ => error!("Privacy service error: {error}")
            }
        }
    }
}

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

struct ModulePublisher {
    sender: ModuleEventSender<PrivacyMessage>
}

impl ModulePublisher {
    const fn new(sender: ModuleEventSender<PrivacyMessage>) -> Self {
        Self {
            sender
        }
    }
}

impl PrivacyEventPublisher for ModulePublisher {
    type SendFuture<'a>
        = Ready<Result<(), PrivacyError>>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<PrivacyService>) -> Self::SendFuture<'_> {
        self.sender.send(PrivacyMessage::Event(event));

        ready(Ok(()))
    }
}

async fn run_start_listening<P>(state: State, publisher: &mut P) -> Result<State, PrivacyError>
where
    P: PrivacyEventPublisher + Send
{
    PrivacyService::start_listening(state, publisher).await
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

    #[tokio::test]
    async fn the_publisher_forwards_service_events_to_the_bus() {
        let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
        let mut receiver = bus.receiver();
        let ctx = context(&bus);

        let mut publisher = ModulePublisher::new(ctx.module_sender(ModuleEvent::Privacy));
        publisher
            .send(ServiceEvent::Error(PrivacyError::WebcamUnavailable))
            .await
            .expect("publishing never fails");

        match receiver.try_recv() {
            Some(BusEvent::Module(ModuleEvent::Privacy(PrivacyMessage::Event(
                ServiceEvent::Error(PrivacyError::WebcamUnavailable)
            )))) => {}
            other => panic!("unexpected event: {other:?}")
        }
    }
}
