use tokio::{runtime::Handle, task::JoinHandle};

use crate::{
    ModuleEventSender,
    services::{
        audio::AudioService, bluetooth::BluetoothService, brightness::BrightnessService,
        idle_inhibitor::IdleInhibitorManager, network::NetworkService, upower::UPowerService
    }
};

pub struct Settings {
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
    pub(super) tasks:           Vec<JoinHandle<()>>
}

impl Default for Settings {
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
            tasks: Vec::new()
        }
    }
}

mod messages;
mod module;
mod update;

#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests;

pub use messages::{Message, SubMenu};
