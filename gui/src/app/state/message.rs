//! The messages every part of the application answers to.

use std::time::Instant;

use hydebar_core::{
    config::{ConfigApplied, ConfigDegradation, ModuleName},
    menu::MenuType,
    modules::{self, tray::TrayMessage},
    position_button::ButtonUIRef,
    tooltip::TooltipInfo
};
use iced::{OutputEvent, SurfaceId as Id};

use super::super::{bus::BusFlushOutcome, shutdown::ShutdownSignal};

#[derive(Debug, Clone)]
pub enum Message {
    None,
    /// A compositor frame callback carrying the frame timestamp.
    Frame(Instant),
    BusFlushed(BusFlushOutcome),
    /// A failed screen measurement asks itself again after a pause.
    RemeasureScreen {
        /// Screen the question is about.
        name: String
    },
    /// The compositor answered a screen geometry question asked off-thread.
    ScreenMeasured {
        /// Screen the question was about.
        name:     String,
        /// The answer, absent when the screen is gone or unnamed.
        geometry: Option<hydebar_core::outputs::scaling::ScreenGeometry>
    },
    /// The compositor answered where the bar's own strip stands.
    StripRowsMeasured(std::collections::HashMap<String, f32>),
    /// The wallpaper in force was read off the disk, ready to draw.
    WallpaperRead(Option<(std::path::PathBuf, iced::widget::image::Handle)>),
    ConfigChanged(ConfigApplied),
    ConfigDegraded(ConfigDegradation),
    /// The process was asked to quit, by a takeover or by the session.
    ///
    /// Handled by taking every surface off the screen before the runtime is
    /// stopped, so a bar that is being replaced leaves nothing behind.
    Shutdown(ShutdownSignal),
    /// Every surface-removal request has been handed to the compositor.
    SurfacesRemoved,
    ToggleMenu(MenuType, Id, ButtonUIRef),
    /// A module of the bar surface was entered or left by the pointer.
    ///
    /// One message serves the tooltip and the attention, because the pointer
    /// resting on a module answers both questions at once: what hint to draw
    /// beside the bar, and what the fast clock should be refreshing.
    ModuleHover {
        /// Bar surface the module is drawn on.
        surface: Id,
        /// Module the pointer entered, or left.
        module:  ModuleName,
        /// Whether the pointer is on the module now.
        entered: bool,
        /// Hint to show while it rests there, absent when it publishes none.
        tooltip: Option<TooltipInfo>
    },
    /// The slow clock came due for the modules resting on the bar.
    PollAtRest,
    /// The fast clock came due for the module being attended.
    PollAttended,
    /// Take down the popups whose time is up.
    ExpirePopups,
    CloseMenu(Id),
    CloseAllMenus,
    /// A press landed on a bar surface while a menu was open.
    ///
    /// Only arms the dismissal: the module the press landed on still gets the
    /// whole click to open or switch its own menu.
    BarPressed,
    /// The press that armed the dismissal completed on a bar surface.
    BarReleased,
    ActivateNavigationMode,
    DeactivateNavigationMode,
    NavigateUp,
    NavigateDown,
    NavigateLeft,
    NavigateRight,
    ActivateFocusedModule,
    OpenLauncher,
    OpenClipboard,
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(TrayMessage),
    Taskbar(modules::taskbar::Message),
    Desk(modules::desk::Message),
    Clock(modules::clock::Message),
    Calendar(modules::calendar::Message),
    HydeMenu(modules::hyde_menu::Message),
    Battery(modules::battery::Message),
    Privacy(modules::privacy::PrivacyMessage),
    ControlCenter(modules::control_center::Message),
    Settings(modules::settings::Message),
    Themes(modules::themes::Message),
    Wallpaper(modules::wallpaper::Message),
    BarLayout(modules::bar_layout::Message),
    MediaPlayer(modules::media_player::Message),
    Notifications(modules::notifications::NotificationsMessage),
    Screenshot(modules::screenshot::ScreenshotMessage),
    Weather(modules::weather::Message),
    OutputEvent(OutputEvent),
    LaunchCommand(String),
    /// An entry of the context menu of a custom module was selected.
    ///
    /// Carries the surface the menu was opened from so it can be dismissed
    /// once the command is on its way.
    CustomMenuAction(Id, String),
    CustomUpdate(std::sync::Arc<str>, modules::custom_module::Message)
}

impl From<modules::control_center::Message> for Message {
    fn from(msg: modules::control_center::Message) -> Self {
        Self::ControlCenter(msg)
    }
}

impl From<modules::taskbar::Message> for Message {
    fn from(msg: modules::taskbar::Message) -> Self {
        Self::Taskbar(msg)
    }
}

impl From<modules::desk::Message> for Message {
    fn from(msg: modules::desk::Message) -> Self {
        Self::Desk(msg)
    }
}

impl From<modules::settings::Message> for Message {
    fn from(msg: modules::settings::Message) -> Self {
        Self::Settings(msg)
    }
}

impl From<modules::themes::Message> for Message {
    fn from(msg: modules::themes::Message) -> Self {
        Self::Themes(msg)
    }
}

impl From<modules::system_info::Message> for Message {
    fn from(msg: modules::system_info::Message) -> Self {
        Self::SystemInfo(msg)
    }
}

impl From<modules::updates::Message> for Message {
    fn from(msg: modules::updates::Message) -> Self {
        Self::Updates(msg)
    }
}

impl From<modules::workspaces::Message> for Message {
    fn from(msg: modules::workspaces::Message) -> Self {
        Self::Workspaces(msg)
    }
}

impl From<modules::notifications::NotificationsMessage> for Message {
    fn from(msg: modules::notifications::NotificationsMessage) -> Self {
        Self::Notifications(msg)
    }
}

impl From<modules::screenshot::ScreenshotMessage> for Message {
    fn from(msg: modules::screenshot::ScreenshotMessage) -> Self {
        Self::Screenshot(msg)
    }
}

impl From<modules::clock::Message> for Message {
    fn from(msg: modules::clock::Message) -> Self {
        Self::Clock(msg)
    }
}

impl From<modules::hyde_menu::Message> for Message {
    fn from(msg: modules::hyde_menu::Message) -> Self {
        Self::HydeMenu(msg)
    }
}
