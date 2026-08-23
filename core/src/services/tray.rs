//! System tray service speaking the status notifier protocol.

use dbus::Layout;
use iced::widget::{image, svg};

pub mod dbus;

mod icon;

pub use icon::icon_from_name;
mod item;
pub use item::StatusNotifierItem;
mod service;
pub use service::{TrayCommand, TrayData, TrayService};
mod watcher;

/// The picture an application registered for its tray entry.
#[derive(Debug, Clone)]
pub enum TrayIcon {
    /// A picture of pixels.
    Image(image::Handle),
    /// A picture of shapes.
    Svg(svg::Handle)
}

/// What the tray watcher has to say.
#[derive(Debug, Clone)]
pub enum TrayEvent {
    /// An application put an icon in the tray.
    Registered(StatusNotifierItem),
    /// An application changed its icon.
    IconChanged(String, TrayIcon),
    /// An application changed its menu.
    MenuLayoutChanged(String, Layout),
    /// An application took its icon away.
    Unregistered(String),
    /// Nothing worth passing on.
    None
}

/// The part of a status notifier name that survives an application restart.
///
/// A name reads `:1.744/org/blueman/sni`: the unique bus prefix changes on
/// every restart of the application, while the object path is the
/// application's own. An item that re-registers after a restart must replace
/// its old self, not stand beside it — a missed unregistration otherwise
/// leaves two icons of one application in the tray.
fn app_identity(name: &str) -> &str {
    name.split_once('/').map_or(name, |(_, path)| path)
}
