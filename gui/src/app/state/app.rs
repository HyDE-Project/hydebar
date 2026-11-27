//! The application state every surface renders from.

use std::{collections::HashMap, sync::Arc};

use hydebar_core::{
    ModuleContext,
    components::icons::IconTheme,
    modules::{
        battery::Battery, calendar::Calendar, clock::Clock, command_button::CommandButton,
        control_center::ControlCenter, custom_module::Custom, desk::Desk, hyde_menu::HydeMenu,
        keyboard_layout::KeyboardLayout, keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer, notifications::Notifications, privacy::Privacy,
        screenshot::Screenshot, settings::Settings, system_info::SystemInfo, taskbar::Taskbar,
        themes::Themes, tray::TrayModule, updates::Updates, wallpaper::Wallpaper,
        weather::Weather, window_title::WindowTitle, workspaces::Workspaces
    },
    outputs::Outputs
};
use hydebar_proto::config::Config;

use super::{focus::Focus, motion::Motion, runtime::Runtime, screen::Screen};

/// Everything the bar is, at this moment, on every screen it stands on.
pub struct App {
    /// The machinery the binary booted the bar with.
    pub(crate) runtime:           Runtime,
    /// The clocks the bar is currently riding.
    pub(crate) motion:            Motion,
    /// What the bar is paying attention to, and what is unfolded around it.
    pub(crate) focus:             Focus,
    /// Theme built from the appearance in force, rebuilt only when it moves.
    ///
    /// Building a theme runs the whole palette derivation; the renderer asks
    /// for the theme on every loop turn, and an idle bar must answer with a
    /// reference-count bump, not five colour cascades.
    pub(crate) theme_cache:       iced::Theme,
    pub(crate) module_context:    ModuleContext,
    pub(crate) icons:             IconTheme,
    /// What the screens have reported about themselves.
    pub(crate) screen:            Screen,
    /// What the machine has been doing for the last few minutes.
    ///
    /// One reading is a number and a run of them is a shape; the canvas draws
    /// the shape beside the number, and nothing else in the bar keeps one.
    pub(crate) history:           super::history::History,
    /// The wallpaper in force, decoded ready to draw, and where it was read.
    ///
    /// Held rather than read where it is drawn: the canvas is built on every
    /// frame of an unfolding, and decoding a picture per frame is a picture
    /// decoded a hundred times for one that was wanted once. The path beside
    /// it is what tells one wallpaper from the next without decoding again.
    pub(crate) wallpaper_preview: Option<(std::path::PathBuf, iced::widget::image::Handle)>,
    /// How the desktop looks, as the pictures it keeps of itself.
    ///
    /// The themes installed here and the wallpapers of the theme in force,
    /// each already centred on the one in force and each already decoded.
    /// Held for the same reason the wallpaper is: the canvas is rebuilt on
    /// every frame of an unfolding and reads no files while it draws.
    pub(crate) looks:             hydebar_core::modules::desk::looks::Looks,
    /// The configuration in force.
    pub config:                   Arc<Config>,
    /// The screens the bar stands on, and the surfaces it holds on them.
    pub outputs:                  Outputs,
    /// Bar entry running the launcher the desktop is configured with.
    pub app_launcher:             CommandButton,
    /// The modules the user wrote, by the key each is named under.
    pub custom:                   HashMap<String, Custom>,
    /// Bar entry counting what is waiting to be installed.
    pub updates:                  Updates,
    /// Bar entry opening the clipboard history.
    pub clipboard:                CommandButton,
    /// Bar entry drawing the compositor’s workspaces.
    pub workspaces:               Workspaces,
    /// Bar entry naming the window holding focus.
    pub window_title:             WindowTitle,
    /// Bar entry reading the machine, and the sample every readout shares.
    pub system_info:              SystemInfo,
    /// Bar entry naming the keyboard layout in force.
    pub keyboard_layout:          KeyboardLayout,
    /// Bar entry naming the compositor submap the keyboard is in.
    pub keyboard_submap:          KeyboardSubmap,
    /// Bar entry holding the icons applications registered.
    pub tray:                     TrayModule,
    /// Bar entry listing the open windows.
    pub taskbar:                  Taskbar,
    /// The canvas the bar unfolds into on a screen holding no window.
    pub desk:                     Desk,
    /// Bar entry stating the date and the time.
    pub clock:                    Clock,
    /// The calendar the clock opens.
    pub calendar:                 Calendar,
    /// Bar entry opening the menu the desktop publishes.
    pub hyde_menu:                HydeMenu,
    /// Bar entry reading the charge and the power source.
    pub battery:                  Battery,
    /// Bar entry warning while the microphone, camera or screen is read.
    pub privacy:                  Privacy,
    /// Bar entry gathering the quick settings, and the services behind them.
    pub control_center:           ControlCenter,
    /// Bar entry naming what is playing.
    pub media_player:             MediaPlayer,
    /// Bar entry holding the notices that arrived.
    pub notifications:            Notifications,
    /// Bar entry taking a picture of the screen.
    pub screenshot:               Screenshot,
    /// Bar entry opening the window the bar is configured from.
    pub settings:                 Settings,
    /// Bar entry choosing the desktop theme, and the one holder of a running
    /// switch.
    pub themes:                   Themes,
    /// Bar entry stepping through the wallpapers of the theme in force.
    pub wallpaper:                Wallpaper,
    /// Bar entry stepping through the layouts the desktop ships.
    pub bar_layout:               hydebar_core::modules::bar_layout::BarLayout,
    /// Bar entry reading the weather where the machine stands.
    pub weather:                  Weather,
    /// The last configuration as the file spelled it, before adoption.
    ///
    /// The cheap gate against reload bursts: a reload whose raw text matches
    /// is finished before the adoption clone and the compositor questions it
    /// asks.
    pub(crate) raw_config:        Option<Arc<Config>>,
    /// Faded and swept themes derived this frame, by quantised key.
    ///
    /// One palette blend serves every island and menu that lands on the
    /// same sixty-fourth of the fade; cleared each frame, so the map holds
    /// a handful of entries and never staleness.
    pub(crate) derived_themes:
        std::cell::RefCell<std::collections::HashMap<(u32, u32), iced::Theme>>
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("config_path", &self.runtime.config_path)
            .field("navigation_mode", &self.focus.navigation_mode)
            .field("focused_module_index", &self.focus.focused_module_index)
            .field("magnification", &self.screen.magnification)
            .field("screen_height", &self.screen.height)
            .finish_non_exhaustive()
    }
}
