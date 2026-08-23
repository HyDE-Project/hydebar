//! Drawing of the notification entry: the bell and what is waiting behind it.

use iced::{
    Alignment, Element,
    widget::{Row, container}
};

use super::Notifications;
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    },
    menu::MenuType,
    modules::OnModulePress
};

impl Notifications {
    /// The bar entry: the bell, with the unread count beside it when
    /// anything is waiting.
    ///
    /// Rendered by the module itself, so the bar layer holds no bell drawing
    /// of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static
    {
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_bell_is_always_on_the_strip_and_opens_its_centre() {
        let notifications = Notifications::default();

        let (_, press) = notifications
            .bar_view::<()>(&IconTheme::default())
            .expect("the bell draws");

        assert!(matches!(
            press,
            Some(OnModulePress::ToggleMenu(MenuType::Notifications))
        ));
    }

    #[test]
    fn waiting_notifications_are_counted_beside_the_bell() {
        let notifications = Notifications {
            unread: 3,
            ..Notifications::default()
        };

        assert!(
            notifications
                .bar_view::<()>(&IconTheme::default())
                .is_some()
        );
    }
}
