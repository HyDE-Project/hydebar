//! The menus a bar module can open.

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum MenuType {
    Updates,
    ControlCenter,
    /// Menu of the standalone audio module.
    Audio,
    /// Menu of the standalone network module.
    Network,
    /// Menu of the standalone bluetooth module.
    Bluetooth,
    /// Menu of the standalone power profile module.
    PowerProfile,
    /// Window configuring the bar itself.
    Settings,
    Tray(String),
    MediaPlayer,
    SystemInfo,
    Notifications,
    Screenshot,
    Calendar,
    /// Context menu of the custom module carrying the given name.
    Custom(String)
}
