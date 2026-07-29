use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Instant};

use flexi_logger::LoggerHandle;
use hydebar_core::{
    ModuleContext,
    components::icons::IconTheme,
    config::{ConfigApplied, ConfigDegradation, ConfigManager, ModuleDef},
    event_bus::{EventReceiver, EventSender},
    menu::MenuType,
    modules::{
        self,
        app_launcher::AppLauncher,
        battery::Battery,
        clipboard::Clipboard,
        clock::Clock,
        control_center::ControlCenter,
        custom_module::Custom,
        idle_inhibitor::IdleInhibitor,
        keyboard_layout::KeyboardLayout,
        keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer,
        notifications::Notifications,
        privacy::Privacy,
        screenshot::Screenshot,
        settings::Settings,
        system_info::SystemInfo,
        tray::{TrayMessage, TrayModule},
        updates::Updates,
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
    config::{Appearance, Config},
    ports::hyprland::HyprlandPort
};
use iced::{Task, event::wayland::OutputEvent, window::Id};
use tokio::runtime::Handle;
use wayland_client::protocol::wl_output::WlOutput;

use super::{bus::BusFlushOutcome, shutdown::ShutdownSignal};

pub struct App {
    pub(super) config_path: PathBuf,
    pub(super) logger: LoggerHandle,
    pub(super) _hyprland: Arc<dyn HyprlandPort>,
    pub(super) config_manager: Arc<ConfigManager>,
    pub(super) bus_receiver: EventReceiver,
    pub(super) last_frame: Option<Instant>,
    pub(super) appearance_transition: AppearanceTransition,
    pub(super) module_context: ModuleContext,
    pub(super) icons: IconTheme,
    /// Factor the screen calls for, folded into every configuration loaded.
    pub(super) magnification: f32,
    /// Sizes the screen calls for, once an output has reported itself.
    pub(super) auto_metrics: Option<AutoMetrics>,
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
    pub battery: Battery,
    pub privacy: Privacy,
    pub control_center: ControlCenter,
    pub media_player: MediaPlayer,
    pub notifications: Notifications,
    pub screenshot: Screenshot,
    pub idle_inhibitor: IdleInhibitor,
    pub settings: Settings,
    pub weather: Weather,
    /// Notifications currently shown as popups.
    pub notification_popups: Vec<notifications_popup::Popup>
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    /// A compositor frame callback carrying the frame timestamp.
    Frame(Instant),
    BusFlushed(BusFlushOutcome),
    ConfigChanged(ConfigApplied),
    ConfigDegraded(ConfigDegradation),
    /// The process was asked to quit, by a takeover or by the session.
    ///
    /// Handled by taking every surface off the screen before the runtime is
    /// stopped, so a bar that is being replaced leaves nothing behind.
    Shutdown(ShutdownSignal),
    ToggleMenu(MenuType, Id, ButtonUIRef),
    /// A module of the bar surface was entered or left by the pointer.
    ///
    /// Carries the hint to show and the placement of the module it belongs to,
    /// or nothing at all once the pointer moves away.
    ModuleTooltip(Id, Option<TooltipInfo>),
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
    Battery(modules::battery::Message),
    Privacy(modules::privacy::PrivacyMessage),
    ControlCenter(modules::control_center::Message),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),
    Notifications(modules::notifications::NotificationsMessage),
    Screenshot(modules::screenshot::ScreenshotMessage),
    Weather(modules::weather::Message),
    OutputEvent((OutputEvent, WlOutput)),
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
        Message::ControlCenter(msg)
    }
}

impl From<modules::settings::Message> for Message {
    fn from(msg: modules::settings::Message) -> Self {
        Message::Settings(msg)
    }
}

impl From<modules::system_info::Message> for Message {
    fn from(msg: modules::system_info::Message) -> Self {
        Message::SystemInfo(msg)
    }
}

impl From<modules::updates::Message> for Message {
    fn from(msg: modules::updates::Message) -> Self {
        Message::Updates(msg)
    }
}

impl From<modules::workspaces::Message> for Message {
    fn from(msg: modules::workspaces::Message) -> Self {
        Message::Workspaces(msg)
    }
}

impl From<modules::notifications::NotificationsMessage> for Message {
    fn from(msg: modules::notifications::NotificationsMessage) -> Self {
        Message::Notifications(msg)
    }
}

impl From<modules::screenshot::ScreenshotMessage> for Message {
    fn from(msg: modules::screenshot::ScreenshotMessage) -> Self {
        Message::Screenshot(msg)
    }
}

impl From<modules::clock::Message> for Message {
    fn from(msg: modules::clock::Message) -> Self {
        Message::Clock(msg)
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
    pub fn appearance(&self) -> &Appearance {
        self.appearance_transition.current()
    }

    /// Returns `config` with the magnification of this screen folded in.
    ///
    /// A configuration read from disk carries the sizes the user wrote, not the
    /// sizes this screen needs; folding the factor in on every load is what
    /// keeps a saved setting from throwing the whole bar back to its unscaled
    /// size.
    pub(super) fn magnified(&self, config: Arc<Config>) -> Arc<Config> {
        if self.magnification <= 1.0 {
            return config;
        }

        let mut magnified = (*config).clone();
        magnified.appearance.magnify(self.magnification);

        Arc::new(magnified)
    }

    /// Appearance the bar renders with.
    ///
    /// The magnification the screen calls for is already folded into the
    /// configuration before the renderer starts, so nothing is added here.
    pub fn scaled_appearance(&self) -> Appearance {
        let appearance = self.config.appearance.clone();

        appearance
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

        let blend_palette = appearance.animations.enabled;
        let resize =
            self.outputs
                .resize(appearance.style, appearance.scale_factor, appearance.height);

        self.appearance_transition
            .set_target(appearance, blend_palette);

        resize
    }

    /// Glyph table to render module icons with this frame.
    ///
    /// Rebuilt whenever the configuration changes so `[icons]` overrides take
    /// effect on a hot reload.
    pub fn icons(&self) -> &IconTheme {
        &self.icons
    }

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
        let mut app = App {
            config_path: config_path.clone(),
            logger,
            _hyprland: hyprland,
            config_manager,
            bus_receiver,
            last_frame: None,
            appearance_transition: AppearanceTransition::new(config.appearance.clone()),
            module_context,
            icons: IconTheme::from_config(&config.icons)
                .with_size(config.appearance.font_size_px()),
            auto_metrics: None,
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
            battery: Battery::default(),
            privacy: Privacy::default(),
            control_center: ControlCenter::default(),
            media_player: MediaPlayer::default(),
            notifications: Notifications::default(),
            screenshot: Screenshot::default(),
            idle_inhibitor: IdleInhibitor,
            settings: Settings::new(config_path.clone()),
            notification_popups: Vec::new(),
            weather: Weather::new(
                config.weather.location.clone(),
                config.weather.api_key.clone(),
                config.weather.use_celsius,
                config.weather.update_interval_minutes
            ),
            config
        };

        app.register_modules();

        if app.config.idle_inhibitor.start_activated {
            app.control_center.set_idle_inhibited(true);
        }

        (app, task)
    }
}

#[cfg(test)]
mod tests {
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
        let mock_port: Arc<dyn HyprlandPort> = mock.clone();

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

        assert!(Arc::ptr_eq(&app._hyprland, &mock_port));
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
}
