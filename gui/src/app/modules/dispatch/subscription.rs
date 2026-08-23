//! The subscription of a bar entry, asked of whatever module owns it.

use hydebar_core::config::ModuleName;
use iced::Subscription;

use crate::app::state::{App, Message};

impl App {
    /// The stream `module_name` produces on its own, if it produces one.
    ///
    /// An entry drawn by a plain function owns nothing to subscribe to, and a
    /// module that publishes through the event bus rather than a stream
    /// answers nothing here — which is all of them but the notification
    /// server.
    pub(crate) fn get_module_subscription(
        &self,
        module_name: &ModuleName
    ) -> Option<Subscription<Message>> {
        self.module_owner(module_name)?.subscription()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_core::config::ModuleName;
    use hydebar_proto::config::{ModuleDef, NotificationSource};

    use super::super::super::super::state::test_support::test_app_with;

    /// The bar with the bell on the strip and `source` chosen for it.
    fn bar_with_the_bell(source: NotificationSource) -> crate::app::state::App {
        test_app_with(|config| {
            config.notifications.source = source;
            config.modules.right = vec![ModuleDef::Single(ModuleName::Notifications)];
        })
    }

    #[test]
    fn a_bell_beside_a_separate_daemon_leaves_the_bus_to_it() {
        let app = bar_with_the_bell(NotificationSource::Daemon);

        assert!(
            app.get_module_subscription(&ModuleName::Notifications)
                .is_none(),
            "drawing the bell must not start the server the configuration declined"
        );
    }

    #[test]
    fn a_bell_the_bar_serves_starts_the_server_behind_it() {
        let app = bar_with_the_bell(NotificationSource::Builtin);

        assert!(
            app.get_module_subscription(&ModuleName::Notifications)
                .is_some(),
            "the bar was asked to serve the bus and has to"
        );
    }
}
