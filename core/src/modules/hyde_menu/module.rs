//! Reading the desktop's menu files when the bar wires the menu up.

use super::{HydeMenu, read};
use crate::{
    ModuleContext,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError}
};

impl<M> Module<M> for HydeMenu
where
    M: 'static
{
    type RegistrationData<'a> = ();

    /// Reads the desktop's menu files off the drawing thread.
    ///
    /// The bar entry wears the glyph the desktop's own module states, and the
    /// glyph is in the same file as the menu — so the entry cannot be drawn
    /// as the desktop means it until that file has been read. Opening the
    /// menu reads it too, but a bar that waited for the first press would
    /// stand there wearing a stand-in glyph until someone pressed it.
    ///
    /// Reported over the bus rather than as a task, because a task handed
    /// back here has nobody to run it: the bar wires its modules up through a
    /// gate that answers with success or failure and nothing else.
    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        let sender = ctx.module_sender(ModuleEvent::HydeMenu);
        self.sender = Some(sender.clone());

        ctx.runtime_handle().spawn(async move {
            match tokio::task::spawn_blocking(read).await {
                Ok(loaded) => sender.send(loaded),
                Err(err) => log::warn!("the desktop's menu files could not be read: {err}")
            }
        });

        Ok(())
    }

    /// Forgets where a read in flight reports back to.
    ///
    /// The tree itself stays: the menu is drawn from what was read, and a
    /// layout that stops hosting the entry stops drawing it anyway.
    fn deregister(&mut self) {
        self.sender = None;
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{num::NonZeroUsize, time::Duration};

    use super::{super::Message, *};
    use crate::event_bus::EventBus;

    #[tokio::test]
    async fn wiring_the_menu_up_reads_the_desktop_without_waiting_for_a_press() {
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let mut receiver = bus.receiver();
        let ctx = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
        let mut menu = HydeMenu::default();

        <HydeMenu as Module<Message>>::register(&mut menu, &ctx, ()).expect("registration");

        let batch = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .expect("the read reports back on its own");

        assert!(
            batch.iter().any(|event| matches!(
                event,
                crate::event_bus::BusEvent::Module(ModuleEvent::HydeMenu(Message::Loaded { .. }))
            )),
            "registration has to report what it read, empty desktop or not"
        );
    }

    #[tokio::test]
    async fn a_layout_that_drops_the_entry_is_reported_to_no_longer() {
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
        let mut menu = HydeMenu::default();

        <HydeMenu as Module<Message>>::register(&mut menu, &ctx, ()).expect("registration");
        <HydeMenu as Module<Message>>::deregister(&mut menu);

        assert!(menu.sender.is_none());
    }
}
