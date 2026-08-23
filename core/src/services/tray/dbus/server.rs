//! The status notifier watcher interface served on the session bus.

#![expect(
    missing_docs,
    reason = "the interface macro writes the proxy beside this one, and a generated item \
              takes no doc comment of ours"
)]

use log::warn;
use zbus::{
    Result, interface,
    message::Header,
    names::{UniqueName, WellKnownName},
    object_server::SignalEmitter
};

pub(super) const NAME: WellKnownName =
    WellKnownName::from_static_str_unchecked("org.kde.StatusNotifierWatcher");
pub(super) const OBJECT_PATH: &str = "/StatusNotifierWatcher";

/// The tray registry the bar serves, so applications have somewhere to
/// register.
#[derive(Debug, Default)]
pub struct StatusNotifierWatcher {
    pub(super) items: Vec<(UniqueName<'static>, String)>
}

/// The interface applications register their tray entries against.
#[interface(
    name = "org.kde.StatusNotifierWatcher",
    proxy(
        gen_blocking = false,
        default_service = "org.kde.StatusNotifierWatcher",
        default_path = "/StatusNotifierWatcher",
    )
)]
impl StatusNotifierWatcher {
    async fn register_status_notifier_item(
        &mut self,
        service: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>
    ) {
        let Some(sender) = header.sender() else {
            warn!("tray registration without a sender, ignoring it");
            return;
        };
        let service = if service.starts_with('/') {
            format!("{sender}{service}")
        } else {
            service.to_string()
        };

        if let Err(err) = Self::status_notifier_item_registered(&emitter, &service).await {
            warn!("failed to announce a tray item registration: {err}");
        }

        self.items.retain(|(_, registered)| registered != &service);
        self.items.push((sender.to_owned(), service));
    }

    #[expect(
        clippy::unused_self,
        clippy::needless_pass_by_ref_mut,
        reason = "the zbus interface macro dispatches D-Bus calls through a method receiver"
    )]
    const fn register_status_notifier_host(&mut self, service: &str) {
        let _ = service;
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.iter().map(|(_, x)| x.clone()).collect()
    }

    #[zbus(property)]
    #[expect(
        clippy::unused_self,
        reason = "the zbus interface macro dispatches D-Bus calls through a method receiver"
    )]
    const fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    #[expect(
        clippy::unused_self,
        reason = "the zbus interface macro dispatches D-Bus calls through a method receiver"
    )]
    const fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        service: &str
    ) -> Result<()>;

    #[zbus(signal)]
    pub(super) async fn status_notifier_item_unregistered(
        emitter: &SignalEmitter<'_>,
        service: &str
    ) -> Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(emitter: &SignalEmitter<'_>) -> Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_unregistered(emitter: &SignalEmitter<'_>) -> Result<()>;
}
