//! Construction of the application and the modules it carries.

use std::{path::PathBuf, sync::Arc};

use flexi_logger::LoggerHandle;
use hydebar_core::{
    ModuleContext,
    animation::HoverFades,
    attention::Attention,
    components::icons::{IconTheme, Icons},
    config::ConfigManager,
    event_bus::{EventReceiver, EventSender},
    modules::{
        battery::Battery, calendar::Calendar, clock::Clock, command_button::CommandButton,
        control_center::ControlCenter, custom_module::Custom, desk::Desk, hyde_menu::HydeMenu,
        keyboard_layout::KeyboardLayout, keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer, notifications::Notifications, privacy::Privacy,
        screenshot::Screenshot, settings::Settings, system_info::SystemInfo, taskbar::Taskbar,
        themes::Themes, tray::TrayModule, updates::Updates, wallpaper::Wallpaper,
        weather::Weather, window_title::WindowTitle, workspaces::Workspaces
    },
    outputs::Outputs,
    style::AppearanceTransition
};
use hydebar_proto::{config::Config, ports::hyprland::HyprlandPort};
use iced::Task;
use tokio::runtime::Handle;

use super::{App, Message};

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
            app_launcher: CommandButton::new(Icons::AppLauncher),
            custom,
            updates: Updates::default(),
            clipboard: CommandButton::new(Icons::Clipboard),
            workspaces: Workspaces::new(Arc::clone(&hyprland_clone), &config.workspaces),
            window_title: WindowTitle::new(Arc::clone(&hyprland_clone), &config.window_title),
            system_info: SystemInfo::default(),
            keyboard_layout: KeyboardLayout::new(Arc::clone(&hyprland_clone)),
            keyboard_submap: KeyboardSubmap::new(Arc::clone(&hyprland_clone)),
            tray: TrayModule::default(),
            taskbar: Taskbar::new(Arc::clone(&hyprland_clone)),
            desk: Desk::new(hyprland_clone),
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
            notification_popups: Vec::new(),
            attention: Attention::default(),
            hover: HoverFades::default(),
            sweep: hydebar_core::style::SweepStyle::default(),
            entrance: hydebar_core::animation::Spring::new(0.0),
            relayout: hydebar_core::animation::Spring::new(1.0),
            flip: std::cell::RefCell::new(hydebar_core::components::flip::FlipMemo::default()),
            greeting: hydebar_core::animation::Spring::new(0.0),
            greeting_raised: Vec::new(),
            greeting_deadline: None,
            hints: hydebar_core::tooltip::Hints::default(),
            greeting_line: String::new(),
            raw_config: None,
            wallpaper_pending: None,
            bar_layout_pending: None,
            derived_themes: std::cell::RefCell::new(std::collections::HashMap::new()),
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
        app.arm_birth_animations();

        if app.config.idle_inhibitor.start_activated {
            app.control_center.set_idle_inhibited(true);
        }

        (app, task)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::num::NonZeroUsize;

    use hydebar_core::{event_bus::EventBus, test_utils::MockHyprlandPort};

    use super::{super::test_support::test_logger, *};

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
}
