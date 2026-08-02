//! D-Bus surface of the tray: the watcher server and the item and menu
//! proxies.

pub mod item;
mod lifecycle;
pub mod menu;
pub mod server;

pub use item::{Icon, StatusNotifierItemProxy};
pub use menu::{DBusMenuProxy, Layout, LayoutProps};
pub use server::{StatusNotifierWatcher, StatusNotifierWatcherProxy};
