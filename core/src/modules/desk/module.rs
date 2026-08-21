//! Module trait wiring for the desk.
//!
//! The desk draws nothing on the bar itself: it owns the canvas surface and
//! only needs the compositor's workspace events to know when to unfold. So it
//! registers a listener and leaves [`Module::view`] to the default.

use std::sync::Arc;

use super::{Desk, Message, listener};
use crate::{
    ModuleContext,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError}
};

impl<M> Module<M> for Desk
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = ();
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::Desk));

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(listener::run(hyprland, sender)));
        }

        Ok(())
    }

    /// Drops the compositor event stream and folds the desk back.
    ///
    /// A desk switched off mid-session must not leave its canvas on the
    /// wallpaper: the state it was drawn from goes with the listener.
    fn deregister(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
        self.bareness = super::bareness::Bareness::default();
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{num::NonZeroUsize, sync::Arc, time::Duration};

    use hydebar_proto::ports::hyprland::HyprlandPort;

    use super::{Desk, Message, Module, ModuleContext};
    use crate::{event_bus::EventBus, test_utils::MockHyprlandPort};

    #[tokio::test]
    async fn registration_publishes_the_state_of_the_screens() {
        let bus = EventBus::new(NonZeroUsize::new(8).expect("capacity"));
        let mut receiver = bus.receiver();
        let ctx = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let port = Arc::new(MockHyprlandPort::default());
        let mut desk = Desk::new(Arc::clone(&port) as Arc<dyn HyprlandPort>);

        <Desk as Module<Message>>::register(&mut desk, &ctx, ()).expect("registration");

        let batch = tokio::time::timeout(Duration::from_secs(2), receiver.recv())
            .await
            .expect("the first answer arrives");

        assert!(!batch.is_empty());

        <Desk as Module<Message>>::deregister(&mut desk);
    }

    #[test]
    fn a_desk_switched_off_folds_back() {
        let port = Arc::new(MockHyprlandPort::default()) as Arc<dyn HyprlandPort>;
        let mut desk = Desk::new(port);

        desk.update(Message::ScreensChanged(super::super::test_bareness()));
        assert!(desk.covers(Some("DP-1")));

        <Desk as Module<Message>>::deregister(&mut desk);
        assert!(!desk.covers(Some("DP-1")));
    }
}
