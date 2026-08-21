//! The application state every surface renders from.

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Instant};

use flexi_logger::LoggerHandle;
use hydebar_core::{
    ModuleContext,
    animation::HoverFades,
    attention::Attention,
    components::icons::IconTheme,
    config::{ConfigManager, ModuleName},
    event_bus::EventReceiver,
    modules::{
        battery::Battery, calendar::Calendar, clock::Clock, command_button::CommandButton,
        control_center::ControlCenter, custom_module::Custom, desk::Desk, hyde_menu::HydeMenu,
        keyboard_layout::KeyboardLayout, keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer, notifications::Notifications, privacy::Privacy,
        screenshot::Screenshot, settings::Settings, system_info::SystemInfo, taskbar::Taskbar,
        themes::Themes, tray::TrayModule, updates::Updates, wallpaper::Wallpaper,
        weather::Weather, window_title::WindowTitle, workspaces::Workspaces
    },
    notifications_popup,
    outputs::{AutoMetrics, Outputs},
    position_button::ButtonUIRef,
    style::AppearanceTransition
};
use hydebar_proto::{
    config::{AppearanceStyle, Config},
    ports::hyprland::HyprlandPort
};
use iced::SurfaceId as Id;

/// How long the greeting stays before it fades on its own.
pub(in crate::app) const GREETING_LIFETIME: std::time::Duration =
    std::time::Duration::from_secs(3);

pub struct App {
    pub(crate) config_path: PathBuf,
    pub(crate) logger: LoggerHandle,
    /// Keeps the injected compositor adapter alive for the app's lifetime;
    /// the outputs facade is meant to adopt it in place of its own client.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "held until the outputs facade adopts the injected port"
        )
    )]
    pub(crate) hyprland: Arc<dyn HyprlandPort>,
    pub(crate) config_manager: Arc<ConfigManager>,
    pub(crate) bus_receiver: EventReceiver,
    pub(crate) last_frame: Option<Instant>,
    pub(crate) appearance_transition: AppearanceTransition,
    /// Theme built from the appearance in force, rebuilt only when it moves.
    ///
    /// Building a theme runs the whole palette derivation; the renderer asks
    /// for the theme on every loop turn, and an idle bar must answer with a
    /// reference-count bump, not five colour cascades.
    pub(crate) theme_cache: iced::Theme,
    pub(crate) module_context: ModuleContext,
    pub(crate) icons: IconTheme,
    /// Factor the screen calls for, folded into every configuration loaded.
    pub(crate) magnification: f32,
    /// Sizes the screen calls for, once an output has reported itself.
    pub(crate) auto_metrics: Option<AutoMetrics>,
    /// Logical height of the screen the bar stands on, once reported.
    pub(crate) screen_height: Option<f32>,
    pub config: Arc<Config>,
    pub outputs: Outputs,
    pub navigation_mode: bool,
    pub focused_module_index: Option<usize>,
    pub app_launcher: CommandButton,
    pub custom: HashMap<String, Custom>,
    pub updates: Updates,
    pub clipboard: CommandButton,
    pub workspaces: Workspaces,
    pub window_title: WindowTitle,
    pub system_info: SystemInfo,
    pub keyboard_layout: KeyboardLayout,
    pub keyboard_submap: KeyboardSubmap,
    pub tray: TrayModule,
    pub taskbar: Taskbar,
    /// The canvas the bar unfolds into on a screen holding no window.
    pub desk: Desk,
    pub clock: Clock,
    pub calendar: Calendar,
    pub hyde_menu: HydeMenu,
    pub battery: Battery,
    pub privacy: Privacy,
    pub control_center: ControlCenter,
    pub media_player: MediaPlayer,
    pub notifications: Notifications,
    pub screenshot: Screenshot,
    pub settings: Settings,
    /// Bar entry choosing the desktop theme, and the one holder of a running
    /// switch.
    pub themes: Themes,
    pub wallpaper: Wallpaper,
    pub bar_layout: hydebar_core::modules::bar_layout::BarLayout,
    pub weather: Weather,
    /// Notifications currently shown as popups.
    pub notification_popups: Vec<notifications_popup::Popup>,
    /// The one module the user is looking at, and the clocks that follow it.
    pub attention: Attention,
    /// Fade of the hover highlight of every module the pointer touched.
    pub hover: HoverFades<ModuleName>,
    /// Signature the current or incoming theme crosses the bar with.
    pub sweep: hydebar_core::style::SweepStyle,
    /// Birth of the bar: the islands ride in on the theme's own wave once.
    pub entrance: hydebar_core::animation::Spring,
    /// Travel of the blocks gliding to a rearranged layout's places.
    pub relayout: hydebar_core::animation::Spring,
    /// The book of seats every module records its place in, per frame.
    pub flip: std::cell::RefCell<hydebar_core::components::flip::FlipMemo>,
    /// Presence of the greeting shown mid-screen while the bar comes up.
    pub greeting: hydebar_core::animation::Spring,
    /// The menu surfaces the greeting has raised, each exactly once.
    pub(crate) greeting_raised: Vec<Id>,
    /// The frame instant past which the greeting lets itself out.
    pub(crate) greeting_deadline: Option<Instant>,
    /// Unfolding of the desk, one spring per screen it may unfold on.
    ///
    /// A spring apiece because the screens answer for themselves: one monitor
    /// may be folding back under a window that just mapped while the other is
    /// still unfolding over a workspace that was cleared.
    pub desk_fades: hydebar_core::animation::HoverFades<Option<String>>,
    /// The one tooltip lifecycle: dwell, warmth and the fade either way.
    pub hints: hydebar_core::tooltip::Hints,
    /// The greeting line, composed once when the greeting is armed.
    pub greeting_line: String,
    /// The last configuration as the file spelled it, before adoption.
    ///
    /// The cheap gate against reload bursts: a reload whose raw text matches
    /// is finished before the adoption clone and the compositor questions it
    /// asks.
    pub(crate) raw_config: Option<Arc<Config>>,
    /// The wallpaper press waiting for its pictures before its window opens.
    pub(crate) wallpaper_pending: Option<(Id, ButtonUIRef)>,
    /// The layout press waiting for its roster before its window opens.
    pub(crate) bar_layout_pending: Option<(Id, ButtonUIRef)>,
    /// Faded and swept themes derived this frame, by quantised key.
    ///
    /// One palette blend serves every island and menu that lands on the
    /// same sixty-fourth of the fade; cleared each frame, so the map holds
    /// a handful of entries and never staleness.
    pub(crate) derived_themes:
        std::cell::RefCell<std::collections::HashMap<(u32, u32), iced::Theme>>,
    /// The layer metrics last stated to the compositor.
    ///
    /// Style, scale bits and height bits: while they stand still, a reload
    /// does not re-state the size and exclusive zone of every bar surface.
    pub(crate) stated_layer_metrics: Option<(AppearanceStyle, u64, Option<u32>)>
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("config_path", &self.config_path)
            .field("navigation_mode", &self.navigation_mode)
            .field("focused_module_index", &self.focused_module_index)
            .field("magnification", &self.magnification)
            .field("screen_height", &self.screen_height)
            .finish_non_exhaustive()
    }
}
