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
