//! Registration of the notification bus the bar serves, and the stream it
//! is served from.

use super::{Notifications, NotificationsMessage};
use crate::{
    ModuleContext,
    event_bus::ModuleEvent,
    modules::{Module, ModuleError}
};

impl<M> Module<M> for Notifications
where
    M: 'static + Clone + From<NotificationsMessage>
{
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        let sender = ctx.module_sender(ModuleEvent::Notifications);
        self.sender = Some(sender);

        Ok(())
    }

    /// Forgets the served store once the bus moves to a separate daemon.
    ///
    /// The D-Bus server itself lives inside the [`subscription`] stream and
    /// dies with it; what survives here is a handle to the storage that
    /// server wrote and the snapshot rendered from it. Kept, they would show
    /// the last notifications of the previous source as if they were live.
    ///
    /// [`subscription`]: Module::subscription
    fn deregister(&mut self) {
        self.sender = None;
        self.service = None;
        self.list.clear();
        self.unread = 0;
        self.dnd = false;
    }

    /// The served bus, but only while the bar was wired to serve it.
    ///
    /// The stream *is* the server: it claims
    /// `org.freedesktop.Notifications` the moment it runs. The bar is asked
    /// for it through the layout, which hosts the bell whatever the
    /// configuration says the source is — so a bell drawn beside
    /// `source = "Daemon"` would take the bus off the session's own daemon
    /// and drop everything it received, the store having been let go in
    /// [`deregister`].
    ///
    /// [`register`] leaves the sender behind exactly while the bar owns the
    /// bus, so answering on it says the same thing once instead of restating
    /// the configuration here.
    ///
    /// [`register`]: Module::register
    /// [`deregister`]: Module::deregister
    fn subscription(&self) -> Option<iced::Subscription<M>> {
        use crate::services::ReadOnlyService;

        self.sender.as_ref()?;

        Some(
            crate::services::notifications::NotificationsService::subscribe()
                .map(NotificationsMessage::Event)
                .map(M::from)
        )
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::event_bus::EventBus;

    /// The bar as it stands before any gate has run.
    fn unwired() -> Notifications {
        Notifications::default()
    }

    /// The bar as it stands once the gate wired it to serve the bus.
    fn wired() -> Notifications {
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());
        let mut notifications = unwired();

        <Notifications as Module<NotificationsMessage>>::register(&mut notifications, &ctx, ())
            .expect("registration");

        notifications
    }

    #[tokio::test]
    async fn a_bar_wired_to_serve_the_bus_serves_it() {
        assert!(
            <Notifications as Module<NotificationsMessage>>::subscription(&wired()).is_some(),
            "the server has to run where the configuration asked for it"
        );
    }

    #[test]
    fn a_bar_that_was_never_wired_does_not_claim_the_bus() {
        assert!(
            <Notifications as Module<NotificationsMessage>>::subscription(&unwired()).is_none(),
            "a bell drawn beside a separate daemon must not take the bus off it"
        );
    }

    #[tokio::test]
    async fn letting_the_bus_go_stops_the_server_with_it() {
        let mut notifications = wired();

        <Notifications as Module<NotificationsMessage>>::deregister(&mut notifications);

        assert!(
            <Notifications as Module<NotificationsMessage>>::subscription(&notifications)
                .is_none(),
            "the configuration moved the bus to a daemon and the bar has to let it go"
        );
    }
}
