use tokio::{runtime::Handle, task::JoinHandle};

use crate::{
    ModuleEventSender,
    services::{
        audio::AudioService, bluetooth::BluetoothService, brightness::BrightnessService,
        idle_inhibitor::IdleInhibitorManager, network::NetworkService, upower::UPowerService
    }
};

pub struct ControlCenter {
    pub(super) audio:           Option<AudioService>,
    pub brightness:             Option<BrightnessService>,
    pub(super) network:         Option<NetworkService>,
    pub(super) bluetooth:       Option<BluetoothService>,
    pub(super) idle_inhibitor:  Option<IdleInhibitorManager>,
    pub sub_menu:               Option<SubMenu>,
    pub(super) upower:          Option<UPowerService>,
    pub(super) password_dialog: Option<(String, String)>,
    pub(super) sender:          Option<ModuleEventSender<Message>>,
    pub(super) runtime:         Option<Handle>,
    pub(super) tasks:           Vec<JoinHandle<()>>,
    pub(super) idle_release:    Option<JoinHandle<()>>,
    /// Read of the nearby networks the attended menu asked for.
    ///
    /// Held so a read still in flight is not started a second time: the
    /// answer takes a bus round trip per access point, which on a
    /// busy band outlasts the cadence the menu refreshes at.
    pub(super) network_poll:    Option<JoinHandle<()>>
}

impl std::fmt::Debug for ControlCenter {
    /// Written by hand because the network service and the idle inhibitor
    /// expose no `Debug` of their own.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlCenter")
            .field("audio", &self.audio)
            .field("brightness", &self.brightness)
            .field("network", &self.network.is_some())
            .field("bluetooth", &self.bluetooth)
            .field("idle_inhibitor", &self.idle_inhibitor.is_some())
            .field("sub_menu", &self.sub_menu)
            .field("upower", &self.upower)
            .field("password_dialog", &self.password_dialog)
            .finish_non_exhaustive()
    }
}

impl ControlCenter {
    /// Whether the shared idle inhibitor currently keeps the session awake.
    ///
    /// Returns `false` when the compositor refused the inhibitor protocol,
    /// so callers render the idle state instead of failing.
    #[must_use]
    pub fn is_idle_inhibited(&self) -> bool {
        self.idle_inhibitor
            .as_ref()
            .is_some_and(IdleInhibitorManager::is_inhibited)
    }

    /// Brings the shared inhibitor to `inhibited`, doing nothing when it is
    /// already there or when the compositor refused the protocol.
    ///
    /// Any pending self release is dropped, so a manual toggle always wins
    /// over a timeout armed by an earlier activation.
    pub fn set_idle_inhibited(&mut self, inhibited: bool) {
        if let Some(release) = self.idle_release.take() {
            release.abort();
        }

        let Some(manager) = self.idle_inhibitor.as_mut() else {
            return;
        };

        if manager.is_inhibited() != inhibited {
            manager.toggle();
        }
    }
}

impl Default for ControlCenter {
    fn default() -> Self {
        let idle_inhibitor = match IdleInhibitorManager::new() {
            Ok(manager) => Some(manager),
            Err(err) => {
                log::warn!("Failed to initialize idle inhibitor: {err}");
                None
            }
        };

        Self {
            audio: None,
            brightness: None,
            network: None,
            bluetooth: None,
            idle_inhibitor,
            sub_menu: None,
            upower: None,
            password_dialog: None,
            sender: None,
            runtime: None,
            tasks: Vec::new(),
            idle_release: None,
            network_poll: None
        }
    }
}

mod messages;
mod module;
mod update;

#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests {
    // TODO: Fix broken tests
    #[cfg(all(test, feature = "enable-broken-tests"))]
    mod tests {
        use std::{
            num::NonZeroUsize,
            sync::{
                Arc,
                atomic::{AtomicBool, Ordering}
            }
        };

        use futures::future;
        use tokio::runtime::Runtime;

        use super::*;
        use crate::{event_bus::EventBus, modules::Module};

        #[test]
        fn register_spawns_event_forwarders() {
            let runtime = Runtime::new().expect("runtime");
            let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
            let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
            let mut settings = ControlCenter::default();

            <ControlCenter as Module<Message>>::register(&mut settings, &ctx, ())
                .expect("register should succeed");

            assert!(settings.sender.is_some());
            assert!(settings.runtime.is_some());
            assert_eq!(settings.tasks.len(), 5);

            for task in settings.tasks.drain(..) {
                task.abort();
            }
        }

        #[test]
        #[ignore = "Timing-sensitive test - needs rework"]
        fn register_aborts_existing_tasks() {
            let runtime = Runtime::new().expect("runtime");
            let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
            let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
            let mut settings = ControlCenter::default();

            let cancelled = Arc::new(AtomicBool::new(false));
            let guard_flag = Arc::clone(&cancelled);

            settings.tasks.push(runtime.spawn(async move {
                struct CancelGuard(Arc<AtomicBool>);

                impl Drop for CancelGuard {
                    fn drop(&mut self) {
                        self.0.store(true, Ordering::SeqCst);
                    }
                }

                let _guard = CancelGuard(guard_flag);

                future::pending::<()>().await;
            }));

            <ControlCenter as Module<Message>>::register(&mut settings, &ctx, ())
                .expect("register should succeed");

            assert!(cancelled.load(Ordering::SeqCst));

            for task in settings.tasks.drain(..) {
                task.abort();
            }
        }
    }
}

pub use messages::{Message, SubMenu};
