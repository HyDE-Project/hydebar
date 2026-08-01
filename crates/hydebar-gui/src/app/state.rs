use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Instant};

use flexi_logger::LoggerHandle;
use hydebar_core::{
    ModuleContext,
    animation::HoverFades,
    attention::Attention,
    components::icons::IconTheme,
    config::{ConfigApplied, ConfigDegradation, ConfigManager, ModuleDef, ModuleName},
    event_bus::{EventReceiver, EventSender},
    menu::MenuType,
    modules::{
        self,
        app_launcher::AppLauncher,
        battery::Battery,
        calendar::Calendar,
        clipboard::Clipboard,
        clock::Clock,
        control_center::ControlCenter,
        custom_module::Custom,
        hyde_menu::HydeMenu,
        idle_inhibitor::IdleInhibitor,
        keyboard_layout::KeyboardLayout,
        keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer,
        notifications::Notifications,
        privacy::Privacy,
        screenshot::Screenshot,
        settings::Settings,
        system_info::SystemInfo,
        themes::Themes,
        tray::{TrayMessage, TrayModule},
        updates::Updates,
        wallpaper::Wallpaper,
        weather::Weather,
        window_title::WindowTitle,
        workspaces::Workspaces
    },
    notifications_popup,
    outputs::{AutoMetrics, Outputs},
    position_button::ButtonUIRef,
    style::AppearanceTransition,
    tooltip::TooltipInfo
};
use hydebar_proto::{
    compositor_look::CompositorLook,
    config::{Appearance, AppearanceStyle, Config},
    ports::hyprland::HyprlandPort
};
use iced::{OutputEvent, SurfaceId as Id, Task};
use tokio::runtime::Handle;

use super::{bus::BusFlushOutcome, shutdown::ShutdownSignal};

/// How long the greeting stays before it fades on its own.
pub(super) const GREETING_LIFETIME: std::time::Duration = std::time::Duration::from_secs(3);

pub struct App {
    pub(super) config_path: PathBuf,
    pub(super) logger: LoggerHandle,
    /// Keeps the injected compositor adapter alive for the app's lifetime;
    /// the outputs facade is meant to adopt it in place of its own client.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "held until the outputs facade adopts the injected port"
        )
    )]
    pub(super) hyprland: Arc<dyn HyprlandPort>,
    pub(super) config_manager: Arc<ConfigManager>,
    pub(super) bus_receiver: EventReceiver,
    pub(super) last_frame: Option<Instant>,
    pub(super) appearance_transition: AppearanceTransition,
    /// Theme built from the appearance in force, rebuilt only when it moves.
    ///
    /// Building a theme runs the whole palette derivation; the renderer asks
    /// for the theme on every loop turn, and an idle bar must answer with a
    /// reference-count bump, not five colour cascades.
    pub(super) theme_cache: iced::Theme,
    pub(super) module_context: ModuleContext,
    pub(super) icons: IconTheme,
    /// Factor the screen calls for, folded into every configuration loaded.
    pub(super) magnification: f32,
    /// Sizes the screen calls for, once an output has reported itself.
    pub(super) auto_metrics: Option<AutoMetrics>,
    /// Logical height of the screen the bar stands on, once reported.
    pub(super) screen_height: Option<f32>,
    pub config: Arc<Config>,
    pub outputs: Outputs,
    pub navigation_mode: bool,
    pub focused_module_index: Option<usize>,
    pub app_launcher: AppLauncher,
    pub custom: HashMap<String, Custom>,
    pub updates: Updates,
    pub clipboard: Clipboard,
    pub workspaces: Workspaces,
    pub window_title: WindowTitle,
    pub system_info: SystemInfo,
    pub keyboard_layout: KeyboardLayout,
    pub keyboard_submap: KeyboardSubmap,
    pub tray: TrayModule,
    pub clock: Clock,
    pub calendar: Calendar,
    pub hyde_menu: HydeMenu,
    pub battery: Battery,
    pub privacy: Privacy,
    pub control_center: ControlCenter,
    pub media_player: MediaPlayer,
    pub notifications: Notifications,
    pub screenshot: Screenshot,
    pub idle_inhibitor: IdleInhibitor,
    pub settings: Settings,
    /// Bar entry choosing the desktop theme, and the one holder of a running
    /// switch.
    pub themes: Themes,
    pub wallpaper: Wallpaper,
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
    /// Presence of the greeting shown mid-screen while the bar comes up.
    pub greeting: hydebar_core::animation::Spring,
    /// Whether the greeting currently holds the menu surfaces raised.
    pub(super) greeting_raised: bool,
    /// The frame instant past which the greeting lets itself out.
    pub(super) greeting_deadline: Option<Instant>,
    /// The one tooltip lifecycle: dwell, warmth and the fade either way.
    pub hints: hydebar_core::tooltip::Hints,
    /// The greeting line, composed once when the greeting is armed.
    pub greeting_line: String,
    /// The last configuration as the file spelled it, before adoption.
    ///
    /// The cheap gate against reload bursts: a reload whose raw text matches
    /// is finished before the adoption clone and the compositor questions it
    /// asks.
    pub(super) raw_config: Option<Arc<Config>>,
    /// The layer metrics last stated to the compositor.
    ///
    /// Style, scale bits and height bits: while they stand still, a reload
    /// does not re-state the size and exclusive zone of every bar surface.
    pub(super) stated_layer_metrics: Option<(AppearanceStyle, u64, Option<u32>)>
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

#[derive(Debug, Clone)]
pub enum Message {
    None,
    /// A compositor frame callback carrying the frame timestamp.
    Frame(Instant),
    BusFlushed(BusFlushOutcome),
    /// The compositor answered a screen geometry question asked off-thread.
    ScreenMeasured {
        /// Screen the question was about.
        name:     String,
        /// The answer, absent when the screen is gone or unnamed.
        geometry: Option<hydebar_core::outputs::scaling::ScreenGeometry>
    },
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
    Clock(modules::clock::Message),
    Calendar(modules::calendar::Message),
    HydeMenu(modules::hyde_menu::Message),
    Battery(modules::battery::Message),
    Privacy(modules::privacy::PrivacyMessage),
    ControlCenter(modules::control_center::Message),
    Settings(modules::settings::Message),
    Themes(modules::themes::Message),
    Wallpaper(modules::wallpaper::Message),
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
    CustomUpdate(String, modules::custom_module::Message)
}

impl From<modules::control_center::Message> for Message {
    fn from(msg: modules::control_center::Message) -> Self {
        Self::ControlCenter(msg)
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

type AppDependencies = (
    LoggerHandle,
    Arc<Config>,
    Arc<ConfigManager>,
    PathBuf,
    Arc<dyn HyprlandPort>,
    EventSender,
    Handle,
    EventReceiver
);

impl App {
    /// Appearance to render with this frame.
    ///
    /// While a config reload is blending this differs from the configured
    /// appearance: colours and opacities lag behind their targets until the
    /// transition settles.
    #[must_use]
    pub const fn appearance(&self) -> &Appearance {
        self.appearance_transition.current()
    }

    /// Rebuilds the cached theme from the appearance in force.
    pub(super) fn rebuild_theme(&mut self) {
        self.theme_cache = hydebar_core::style::hydebar_theme(self.appearance());
    }

    /// Returns `config` restated for the screen the bar runs on.
    ///
    /// A configuration read from disk carries the sizes the user wrote, not the
    /// sizes this screen needs and nothing the compositor knows; a reload that
    /// only folded the magnification in would drop the window gap the
    /// outermost islands line up with and fall back to the font-derived
    /// margin, which is why the whole restatement lives in one place and both
    /// the first load and every reload after it go through it.
    pub(super) fn adopted(&self, config: &Config) -> Arc<Config> {
        self.adopted_with(config, &CompositorLook::read())
    }

    /// Restates `config` against a compositor look the caller already has.
    fn adopted_with(&self, config: &Config, look: &CompositorLook) -> Arc<Config> {
        let mut adopted = config.clone();
        adopted.appearance.adopt_screen(self.magnification, look);

        Arc::new(adopted)
    }

    /// Appearance the bar renders with.
    ///
    /// The magnification the screen calls for is already folded into the
    /// configuration before the renderer starts, so nothing is added here.
    #[must_use]
    pub fn scaled_appearance(&self) -> Appearance {
        self.config.appearance.clone()
    }

    /// Rebuilds everything derived from the appearance after the sizes changed.
    ///
    /// The surfaces are re-stated as well: the height of a layer surface is
    /// fixed when it is created, so a bar that changed height has to tell the
    /// compositor, otherwise the strip it occupies keeps the old size while its
    /// contents are drawn at the new one.
    pub(super) fn refresh_appearance(&mut self) -> Task<Message> {
        let appearance = self.scaled_appearance();

        hydebar_core::components::scale::set_base(appearance.font_size_px());

        self.icons =
            IconTheme::from_config(&self.config.icons).with_size(appearance.font_size_px());
        self.rebuild_theme();

        let blend_palette = appearance.animations.enabled;

        // the compositor is only told about the strip when the strip moved:
        // a reload that recoloured the bar must not re-state the size and
        // exclusive zone of every surface it did not touch
        let metrics = (
            appearance.style,
            appearance.scale_factor.to_bits(),
            appearance.height.map(f32::to_bits)
        );
        let resize = if self.stated_layer_metrics == Some(metrics) {
            Task::none()
        } else {
            self.stated_layer_metrics = Some(metrics);
            self.outputs
                .resize(appearance.style, appearance.scale_factor, appearance.height)
        };

        let incoming = self
            .themes
            .switching()
            .or_else(|| self.themes.hyde().theme.as_deref());
        self.sweep = hydebar_core::style::SweepStyle::of(incoming, &appearance);
        self.appearance_transition
            .restyle(self.sweep.response, self.sweep.damping);

        self.appearance_transition
            .set_target(appearance, blend_palette);

        resize
    }

    /// Glyph table to render module icons with this frame.
    ///
    /// Rebuilt whenever the configuration changes so `[icons]` overrides take
    /// effect on a hot reload.
    #[must_use]
    pub const fn icons(&self) -> &IconTheme {
        &self.icons
    }

    #[must_use]
    pub fn get_all_modules_count(&self) -> usize {
        let count_modules = |modules_def: &[ModuleDef]| -> usize {
            modules_def
                .iter()
                .map(|def| match def {
                    ModuleDef::Single(_) => 1,
                    ModuleDef::Group(group) => group.len()
                })
                .sum()
        };

        count_modules(&self.config.modules.left)
            + count_modules(&self.config.modules.center)
            + count_modules(&self.config.modules.right)
    }

    pub fn new(
        (
            logger,
            config,
            config_manager,
            config_path,
            hyprland,
            event_sender,
            runtime_handle,
            bus_receiver
        ): AppDependencies
    ) -> (Self, Task<Message>) {
        let (outputs, task) = Outputs::new(config.appearance.style, config.position, &config);

        let custom = config
            .custom_modules
            .iter()
            .map(|o| (o.name.clone(), Custom::default()))
            .collect();
        let module_context = ModuleContext::new(event_sender, runtime_handle);
        let hyprland_clone = Arc::clone(&hyprland);
        let mut app = Self {
            config_path: config_path.clone(),
            logger,
            hyprland,
            config_manager,
            bus_receiver,
            last_frame: None,
            appearance_transition: AppearanceTransition::new(config.appearance.clone()),
            theme_cache: hydebar_core::style::hydebar_theme(&config.appearance),
            module_context,
            icons: IconTheme::from_config(&config.icons)
                .with_size(config.appearance.font_size_px()),
            auto_metrics: None,
            screen_height: None,
            magnification: hydebar_core::components::scale::screen_factor(),
            outputs,
            navigation_mode: false,
            focused_module_index: None,
            app_launcher: AppLauncher,
            custom,
            updates: Updates::default(),
            clipboard: Clipboard,
            workspaces: Workspaces::new(Arc::clone(&hyprland_clone), &config.workspaces),
            window_title: WindowTitle::new(Arc::clone(&hyprland_clone), &config.window_title),
            system_info: SystemInfo::default(),
            keyboard_layout: KeyboardLayout::new(Arc::clone(&hyprland_clone)),
            keyboard_submap: KeyboardSubmap::new(hyprland_clone),
            tray: TrayModule::default(),
            clock: Clock::default(),
            calendar: Calendar::default(),
            hyde_menu: HydeMenu::default(),
            battery: Battery::default(),
            privacy: Privacy::default(),
            control_center: ControlCenter::default(),
            media_player: MediaPlayer::default(),
            notifications: Notifications::default(),
            screenshot: Screenshot::default(),
            idle_inhibitor: IdleInhibitor,
            settings: Settings::new(config_path),
            themes: Themes::new(),
            wallpaper: Wallpaper::new(),
            notification_popups: Vec::new(),
            attention: Attention::default(),
            hover: HoverFades::default(),
            sweep: hydebar_core::style::SweepStyle::default(),
            entrance: hydebar_core::animation::Spring::new(0.0),
            greeting: hydebar_core::animation::Spring::new(0.0),
            greeting_raised: false,
            greeting_deadline: None,
            hints: hydebar_core::tooltip::Hints::default(),
            greeting_line: String::new(),
            raw_config: None,
            stated_layer_metrics: None,
            weather: Weather::new(
                config.weather.location.clone(),
                config.weather.api_key.clone(),
                config.weather.use_celsius,
                config.weather.update_interval_minutes
            ),
            config
        };

        app.register_modules();

        app.sweep = hydebar_core::style::SweepStyle::of(
            app.themes.hyde().theme.as_deref(),
            &app.config.appearance
        );
        app.entrance = hydebar_core::animation::Spring::new(0.0)
            .with_response(app.sweep.response)
            .with_damping_ratio(app.sweep.damping);

        if app.config.appearance.animations.enabled {
            app.entrance.set_target(1.0);

            if app.config.appearance.greeting
                && hydebar_core::components::greeting::claim_first_entry()
            {
                app.greeting_line = hydebar_core::components::greeting::current();
                app.greeting = hydebar_core::animation::Spring::new(0.0)
                    .with_response(hydebar_core::animation::STANDARD);
                app.greeting.set_target(1.0);
            }
        } else {
            app.entrance.snap_to(1.0);
        }

        if app.config.idle_inhibitor.start_activated {
            app.control_center.set_idle_inhibited(true);
        }

        (app, task)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::{num::NonZeroUsize, sync::OnceLock};

    use flexi_logger::LoggerHandle;
    use hydebar_core::{config::ConfigManager, event_bus::EventBus, test_utils::MockHyprlandPort};
    use hydebar_proto::ports::hyprland::HyprlandPort;

    use super::*;

    fn test_logger() -> LoggerHandle {
        static LOGGER: OnceLock<LoggerHandle> = OnceLock::new();
        LOGGER
            .get_or_init(|| {
                flexi_logger::Logger::try_with_env_or_str("off")
                    .expect("failed to configure test logger")
                    .start()
                    .expect("failed to start test logger")
            })
            .clone()
    }

    #[test]
    fn app_stores_injected_hyprland_port() {
        let logger = test_logger();
        let config = Config::default();
        let path = PathBuf::new();
        let mock = Arc::new(MockHyprlandPort::default());
        let mock_port: Arc<dyn HyprlandPort> = mock;

        let config_manager = Arc::new(ConfigManager::new(config.clone()));
        let capacity = NonZeroUsize::new(16).expect("non-zero");
        let bus = EventBus::new(capacity);
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let event_sender = bus.sender();
        let runtime_handle = runtime.handle().clone();
        let bus_receiver = bus.receiver();

        let (app, _) = App::new((
            logger,
            Arc::new(config),
            Arc::clone(&config_manager),
            path,
            Arc::clone(&mock_port),
            event_sender,
            runtime_handle,
            bus_receiver
        ));

        assert!(Arc::ptr_eq(&app.hyprland, &mock_port));
    }

    #[test]
    fn keyboard_layout_change_triggers_port_call() {
        let logger = test_logger();
        let config = Config::default();
        let path = PathBuf::new();
        let mock = Arc::new(MockHyprlandPort::default());
        let mock_port: Arc<dyn HyprlandPort> = mock.clone();

        let config_manager = Arc::new(ConfigManager::new(config.clone()));
        let capacity = NonZeroUsize::new(16).expect("non-zero");
        let bus = EventBus::new(capacity);
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let event_sender = bus.sender();
        let runtime_handle = runtime.handle().clone();
        let bus_receiver = bus.receiver();

        let (mut app, _) = App::new((
            logger,
            Arc::new(config),
            Arc::clone(&config_manager),
            path,
            mock_port,
            event_sender,
            runtime_handle,
            bus_receiver
        ));

        let _ = app.update(Message::KeyboardLayout(
            hydebar_core::modules::keyboard_layout::Message::ChangeLayout
        ));

        assert_eq!(mock.switch_layout_calls(), 1);
    }

    fn test_app(magnification: f32) -> App {
        let logger = test_logger();
        let config = Config::default();
        let mock_port: Arc<dyn HyprlandPort> = Arc::new(MockHyprlandPort::default());
        let config_manager = Arc::new(ConfigManager::new(config.clone()));
        let capacity = NonZeroUsize::new(16).expect("non-zero");
        let bus = EventBus::new(capacity);
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let event_sender = bus.sender();
        let runtime_handle = runtime.handle().clone();
        let bus_receiver = bus.receiver();

        let (mut app, _) = App::new((
            logger,
            Arc::new(config),
            config_manager,
            PathBuf::new(),
            mock_port,
            event_sender,
            runtime_handle,
            bus_receiver
        ));
        app.magnification = magnification;

        app
    }

    fn window_look() -> CompositorLook {
        CompositorLook {
            rounding: Some(3.0),
            gaps_out: Some(8.0),
            gaps_in: Some(3.0),
            animations: Some(true),
            blur: Some(true),
            ..CompositorLook::default()
        }
    }

    #[test]
    fn a_reloaded_config_keeps_the_islands_at_the_window_gap() {
        let app = test_app(2.0);
        let mut config = Config::default();
        config.appearance.font_size = Some(10.0);
        config.appearance.side_padding = None;

        let reloaded = app.adopted_with(&config, &window_look());

        assert_eq!(reloaded.appearance.side_padding, Some(8.0));
        assert_eq!(reloaded.appearance.bar_padding()[1], 8.0);
    }

    #[test]
    fn reloading_over_and_over_never_moves_the_islands() {
        let app = test_app(2.0);
        let mut config = Config::default();
        config.appearance.font_size = Some(10.0);

        let once = app.adopted_with(&config, &window_look());
        let twice = app.adopted_with(&config, &window_look());

        assert_eq!(
            once.appearance.bar_padding(),
            twice.appearance.bar_padding()
        );
        assert_eq!(once.appearance.font_size, twice.appearance.font_size);
    }

    #[test]
    fn an_unmagnified_bar_is_restated_all_the_same() {
        let app = test_app(1.0);
        let config = Config::default();

        let reloaded = app.adopted_with(&config, &window_look());

        assert_eq!(reloaded.appearance.side_padding, Some(8.0));
    }
}
