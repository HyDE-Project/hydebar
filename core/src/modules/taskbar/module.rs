//! Registration of the compositor stream the taskbar strip owns.
//!
//! Background updates are delivered via the shared module event sender: the
//! listener publishes client lists onto the event bus and the bar folds them
//! in through [`Taskbar::update`](super::Taskbar::update).

use std::sync::Arc;

use super::{Taskbar, listener};
use crate::{
    ModuleContext,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError}
};

impl<M> Module<M> for Taskbar
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
        self.sender = Some(ctx.module_sender(ModuleEvent::Taskbar));
        self.runtime = Some(ctx.runtime_handle().clone());

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(listener::run(hyprland, sender)));
        }

        Ok(())
    }

    /// Drops the compositor event stream once the strip leaves the bar.
    ///
    /// The listener re-reads the whole client list on every window event; a
    /// layout without a taskbar would pay that for nothing.
    fn deregister(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        self.sender = None;
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{num::NonZeroUsize, sync::Arc, time::Duration};

    use hydebar_proto::ports::hyprland::HyprlandPort;

    use super::{Module, ModuleContext, Taskbar};
    use crate::{
        event_bus::EventBus, modules::taskbar::test_client, test_utils::MockHyprlandPort
    };

    #[tokio::test]
    async fn registration_publishes_the_first_snapshot() {
        let bus = EventBus::new(NonZeroUsize::new(8).expect("capacity"));
        let mut receiver = bus.receiver();
        let ctx = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let port = Arc::new(MockHyprlandPort::default());
        *port.clients_snapshot.lock().expect("clients lock") = vec![test_client("0x1", true)];

        let mut taskbar = Taskbar::new(Arc::clone(&port) as Arc<dyn HyprlandPort>);
        <Taskbar as Module<()>>::register(&mut taskbar, &ctx, ()).expect("registration");

        let batch = tokio::time::timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("the snapshot arrives");

        assert!(!batch.is_empty());

        <Taskbar as Module<()>>::deregister(&mut taskbar);
    }
}
