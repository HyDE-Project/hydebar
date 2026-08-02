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

#[derive(Debug, Clone)]
pub enum TrayIcon {
    Image(image::Handle),
    Svg(svg::Handle)
}

#[derive(Debug, Clone)]
pub enum TrayEvent {
    Registered(StatusNotifierItem),
    IconChanged(String, TrayIcon),
    MenuLayoutChanged(String, Layout),
    Unregistered(String),
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
