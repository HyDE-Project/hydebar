//! The bar's entries: every module the strip can carry, each held in the
//! one place that outlives the frames it is drawn in.
//!
//! These are the living modules, not their messages: the messages travel
//! through `app::modules`, while what they act upon stands here — a clock
//! that knows the time it last showed, a tray that knows the icons it was
//! given, a desk that knows which screens it covers.

use std::{collections::HashMap, sync::Arc};

use hydebar_core::{
    components::icons::Icons,
    modules::{
        battery::Battery, calendar::Calendar, clock::Clock, command_button::CommandButton,
        control_center::ControlCenter, custom_module::Custom, desk::Desk, hyde_menu::HydeMenu,
        keyboard_layout::KeyboardLayout, keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer, notifications::Notifications, privacy::Privacy,
        screenshot::Screenshot, settings::Settings, system_info::SystemInfo, taskbar::Taskbar,
        themes::Themes, tray::TrayModule, updates::Updates, wallpaper::Wallpaper,
        weather::Weather, window_title::WindowTitle, workspaces::Workspaces
    }
};
use hydebar_proto::{config::Config, ports::hyprland::HyprlandPort};

/// Every module the bar carries, by the name its strip entry answers to.
pub struct Modules {
    /// Bar entry running the launcher the desktop is configured with.
    pub app_launcher:    CommandButton,
    /// The modules the user wrote, by the key each is named under.
    pub custom:          HashMap<String, Custom>,
    /// Bar entry counting what is waiting to be installed.
    pub updates:         Updates,
    /// Bar entry opening the clipboard history.
    pub clipboard:       CommandButton,
    /// Bar entry drawing the compositor’s workspaces.
    pub workspaces:      Workspaces,
    /// Bar entry naming the window holding focus.
    pub window_title:    WindowTitle,
    /// Bar entry reading the machine, and the sample every readout shares.
    pub system_info:     SystemInfo,
    /// Bar entry naming the keyboard layout in force.
    pub keyboard_layout: KeyboardLayout,
    /// Bar entry naming the compositor submap the keyboard is in.
    pub keyboard_submap: KeyboardSubmap,
    /// Bar entry holding the icons applications registered.
    pub tray:            TrayModule,
    /// Bar entry listing the open windows.
    pub taskbar:         Taskbar,
    /// The canvas the bar unfolds into on a screen holding no window.
    pub desk:            Desk,
    /// Bar entry stating the date and the time.
    pub clock:           Clock,
    /// The calendar the clock opens.
    pub calendar:        Calendar,
    /// Bar entry opening the menu the desktop publishes.
    pub hyde_menu:       HydeMenu,
    /// Bar entry reading the charge and the power source.
    pub battery:         Battery,
    /// Bar entry warning while the microphone, camera or screen is read.
    pub privacy:         Privacy,
    /// Bar entry gathering the quick settings, and the services behind them.
    pub control_center:  ControlCenter,
    /// Bar entry naming what is playing.
    pub media_player:    MediaPlayer,
    /// Bar entry holding the notices that arrived.
    pub notifications:   Notifications,
    /// Bar entry taking a picture of the screen.
    pub screenshot:      Screenshot,
    /// Bar entry opening the window the bar is configured from.
    pub settings:        Settings,
    /// Bar entry choosing the desktop theme, and the one holder of a running
    /// switch.
    pub themes:          Themes,
    /// Bar entry stepping through the wallpapers of the theme in force.
    pub wallpaper:       Wallpaper,
    /// Bar entry stepping through the layouts the desktop ships.
    pub bar_layout:      hydebar_core::modules::bar_layout::BarLayout,
    /// Bar entry reading the weather where the machine stands.
    pub weather:         Weather
}

/// Builds every entry the born bar will carry.
///
/// The compositor-bound entries share one handle to the port: each module
/// asks the compositor for its own corner of the screen, and they all ask
/// the same compositor.
pub(super) fn initial_modules(
    config: &Arc<Config>,
    hyprland: &Arc<dyn HyprlandPort>,
    custom: HashMap<String, Custom>,
    config_path: std::path::PathBuf
) -> Modules {
    Modules {
        app_launcher: CommandButton::new(Icons::AppLauncher),
        custom,
        updates: Updates::default(),
        clipboard: CommandButton::new(Icons::Clipboard),
        workspaces: Workspaces::new(Arc::clone(hyprland), &config.workspaces),
        window_title: WindowTitle::new(Arc::clone(hyprland), &config.window_title),
        system_info: SystemInfo::default(),
        keyboard_layout: KeyboardLayout::new(Arc::clone(hyprland)),
        keyboard_submap: KeyboardSubmap::new(Arc::clone(hyprland)),
        tray: TrayModule::default(),
        taskbar: Taskbar::new(Arc::clone(hyprland)),
        desk: Desk::new(Arc::clone(hyprland)),
        clock: Clock::default(),
        calendar: Calendar::default(),
        hyde_menu: HydeMenu::default(),
        battery: Battery::default(),
        privacy: Privacy::default(),
        control_center: ControlCenter::default(),
        media_player: MediaPlayer::default(),
        notifications: Notifications::default(),
        screenshot: Screenshot::default(),
        settings: Settings::new(config_path),
        themes: Themes::new(),
        wallpaper: Wallpaper::new(),
        bar_layout: hydebar_core::modules::bar_layout::BarLayout::new(),
        weather: Weather::new(
            config.weather.location.clone(),
            config.weather.api_key.clone(),
            config.weather.use_celsius,
            config.weather.update_interval_minutes
        )
    }
}
