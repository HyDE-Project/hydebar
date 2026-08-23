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

/// Everything the bar answers to, from a press to a service speaking up.
#[derive(Debug, Clone)]
pub enum Message {
    /// Nothing happened; the answer of a handler with nothing to say.
    None,
    /// A compositor frame callback carrying the frame timestamp.
    Frame(Instant),
    /// The event bus was drained, and this is what came out of it.
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
    /// The pictures the desktop keeps of its own look were read off the disk.
    LooksRead(Box<hydebar_core::modules::desk::looks::Looks>),
    /// The configuration file was read again and the new one applies.
    ConfigChanged(ConfigApplied),
    /// The configuration file was read again and could not be used.
    ConfigDegraded(ConfigDegradation),
    /// The process was asked to quit, by a takeover or by the session.
    ///
    /// Handled by taking every surface off the screen before the runtime is
    /// stopped, so a bar that is being replaced leaves nothing behind.
    Shutdown(ShutdownSignal),
    /// Every surface-removal request has been handed to the compositor.
    SurfacesRemoved,
    /// A press asked for a menu, from this surface and this button.
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
    /// Close whatever menu this surface holds open.
    CloseMenu(Id),
    /// Close every open menu on every surface.
    CloseAllMenus,
    /// A press landed on a bar surface while a menu was open.
    ///
    /// Only arms the dismissal: the module the press landed on still gets the
    /// whole click to open or switch its own menu.
    BarPressed,
    /// The press that armed the dismissal completed on a bar surface.
    BarReleased,
    /// Hand the keyboard to the bar.
    ActivateNavigationMode,
    /// Give the keyboard back.
    DeactivateNavigationMode,
    /// Move the keyboard selection towards the top.
    NavigateUp,
    /// Move the keyboard selection towards the bottom.
    NavigateDown,
    /// Move the keyboard selection towards the leading edge.
    NavigateLeft,
    /// Move the keyboard selection towards the trailing edge.
    NavigateRight,
    /// Press the module the keyboard selection rests on.
    ActivateFocusedModule,
    /// Run the command the launcher entry is configured with.
    OpenLauncher,
    /// Run the command the clipboard entry is configured with.
    OpenClipboard,
    /// Something happened in the update count.
    Updates(modules::updates::Message),
    /// Something happened in the workspaces.
    Workspaces(modules::workspaces::Message),
    /// Something happened in the focused window.
    WindowTitle(modules::window_title::Message),
    /// Something happened in the machine readout.
    SystemInfo(modules::system_info::Message),
    /// Something happened in the keyboard layout.
    KeyboardLayout(modules::keyboard_layout::Message),
    /// Something happened in the keyboard submap.
    KeyboardSubmap(modules::keyboard_submap::Message),
    /// Something happened in the system tray.
    Tray(TrayMessage),
    /// Something happened in the window list.
    Taskbar(modules::taskbar::Message),
    /// Something happened in the canvas.
    Desk(modules::desk::Message),
    /// Something happened in the clock.
    Clock(modules::clock::Message),
    /// Something happened in the calendar.
    Calendar(modules::calendar::Message),
    /// Something happened in the desktop menu.
    HydeMenu(modules::hyde_menu::Message),
    /// Something happened in the battery.
    Battery(modules::battery::Message),
    /// Something happened in the privacy indicators.
    Privacy(modules::privacy::PrivacyMessage),
    /// Something happened in the quick settings.
    ControlCenter(modules::control_center::Message),
    /// Something happened in the settings window.
    Settings(modules::settings::Message),
    /// Something happened in the themes.
    Themes(modules::themes::Message),
    /// Something happened in the wallpaper.
    Wallpaper(modules::wallpaper::Message),
    /// Something happened in the bar layout.
    BarLayout(modules::bar_layout::Message),
    /// Something happened in the media player.
    MediaPlayer(modules::media_player::Message),
    /// Something happened in the notices.
    Notifications(modules::notifications::NotificationsMessage),
    /// Something happened in the screenshot tool.
    Screenshot(modules::screenshot::ScreenshotMessage),
    /// Something happened in the weather.
    Weather(modules::weather::Message),
    /// The compositor said something about a screen.
    OutputEvent(OutputEvent),
    /// Run this shell command on the bar's behalf.
    LaunchCommand(String),
    /// An entry of the context menu of a custom module was selected.
    ///
    /// Carries the surface the menu was opened from so it can be dismissed
    /// once the command is on its way.
    CustomMenuAction(Id, String),
    /// A module the user wrote had something to say, named by its key.
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
