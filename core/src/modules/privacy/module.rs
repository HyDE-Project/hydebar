//! Registration of the listener loop the privacy indicator owns.
//!
//! The loop keeps the privacy service connected, backing off after failures,
//! so a `PipeWire` that went away is picked up again rather than leaving the
//! indicator blind.

use super::{
    Privacy, PrivacyMessage,
    publisher::{ModulePublisher, run_start_listening}
};
use crate::{
    ModuleContext,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError},
    services::{ServiceEvent, privacy::State}
};

impl<M> Module<M> for Privacy
where
    M: 'static
{
    type ViewData<'a> = ();
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
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
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
