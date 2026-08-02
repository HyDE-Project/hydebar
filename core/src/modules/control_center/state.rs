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
    pub(super) brightness:      Option<BrightnessService>,
    pub(super) network:         Option<NetworkService>,
    pub(super) bluetooth:       Option<BluetoothService>,
    pub(super) idle_inhibitor:  Option<IdleInhibitorManager>,
    pub(super) sub_menu:        Option<SubMenu>,
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
    /// Prepares the standalone audio window: it opens on the sink list.
    pub const fn open_audio_menu(&mut self) {
        self.sub_menu = Some(SubMenu::Sinks);
    }

    /// Prepares the standalone bluetooth window on its device list.
    pub const fn open_bluetooth_menu(&mut self) {
        self.sub_menu = Some(SubMenu::Bluetooth);
    }

    /// Folds whatever submenu a previous visit left open.
    pub const fn close_submenu(&mut self) {
        self.sub_menu = None;
    }

    /// Asks the backlight for a fresh reading as the window opens.
    ///
    /// The window shows a slider over a value that may have moved from a
    /// keybinding since the last visit; opening is the moment to ask.
    pub fn refresh_brightness(&mut self) -> iced::Task<Message> {
        use crate::services::{Service, brightness::BrightnessCommand};

        self.brightness
            .as_mut()
            .map_or_else(iced::Task::none, |brightness| {
                brightness.command(BrightnessCommand::Refresh).map(|event| {
                    Message::Brightness(super::BrightnessMessage::Event(Box::new(event)))
                })
            })
    }

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

pub use messages::{Message, SubMenu};
