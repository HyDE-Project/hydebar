//! Module trait wiring for the notifications module.

use iced::{
    Alignment, Element,
    widget::{Row, container}
};

use super::{Notifications, NotificationsMessage};
use crate::{
    ModuleContext,
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    },
    event_bus::ModuleEvent,
    menu::MenuType,
    modules::{Module, ModuleError, OnModulePress}
};

impl<M> Module<M> for Notifications
where
    M: 'static + Clone + From<NotificationsMessage>
{
    type ViewData<'a> = &'a IconTheme;
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

    fn subscription(&self) -> Option<iced::Subscription<M>> {
        use crate::services::ReadOnlyService;

        Some(
            crate::services::notifications::NotificationsService::subscribe()
                .map(NotificationsMessage::Event)
                .map(M::from)
        )
    }

    /// Render notification icon with unread count.
    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        let content = if self.unread > 0 {
            Row::new()
                .push(icon(icons, Icons::Bell))
                .push(text(self.unread))
                .spacing(scale::icon_gap())
                .align_y(Alignment::Center)
        } else {
            Row::new().push(icon(icons, Icons::Bell))
        };

        Some((
            container(content).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Notifications))
        ))
    }
}
