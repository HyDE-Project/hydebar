//! Construction of the application and the modules it carries.

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use flexi_logger::LoggerHandle;
use hydebar_core::{
    ModuleContext,
    animation::HoverFades,
    attention::Attention,
    components::icons::IconTheme,
    config::ConfigManager,
    event_bus::{EventReceiver, EventSender},
    modules::custom_module::Custom,
    outputs::Outputs,
    style::AppearanceTransition
};
use hydebar_proto::{
    config::{Appearance, Config},
    ports::hyprland::HyprlandPort
};
use iced::Task;
use tokio::runtime::Handle;

use super::{
    App, Message, entries::initial_modules, focus::Focus, motion::Motion, runtime::Runtime,
    screen::Screen
};

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

/// The modules the user wrote themselves, each holding what the desktop's own
/// menu for it offers.
///
/// The desktop's files are read here, once, rather than where the canvas draws
/// them: a module that is a button and nothing else opens into a row of the
/// choices behind it, and a row rebuilt off the filesystem on every frame of
/// an unfolding would be a read per block per frame.
fn custom_modules(config: &Config) -> HashMap<String, Custom> {
    config
        .custom_modules
        .iter()
        .map(|module| {
            (
                module.name.clone(),
                Custom::with_choices(hydebar_core::modules::hyde_menu::desktop_choices(
                    &module.name
                ))
            )
        })
        .collect()
}

fn initial_screen() -> Screen {
    Screen {
        auto_metrics:         None,
        height:               None,
        width:                None,
        sizes:                HashMap::new(),
        strip_rows:           HashMap::new(),
        magnification:        hydebar_core::components::scale::screen_factor(),
        stated_layer_metrics: None
    }
}

/// The clocks as the bar is born: still, except the strip's background,
/// which starts open because the first canvas has nothing to come back from.
fn initial_motion(appearance: Appearance) -> Motion {
    Motion {
        last_frame:            None,
        appearance_transition: AppearanceTransition::new(appearance),
        sweep:                 hydebar_core::style::SweepStyle::default(),
        entrance:              hydebar_core::animation::Spring::new(0.0),
        relayout:              hydebar_core::animation::Spring::new(1.0),
        flip:                  std::cell::RefCell::new(
            hydebar_core::components::flip::FlipMemo::default()
        ),
        desk_clocks:           HashMap::new(),
        desk_returning:        {
            let mut painted = hydebar_core::animation::Unfold::default();
            painted.open();
            painted
        }
    }
}

/// The attention as the bar is born: nothing held, nothing unfolded, the
/// greeting armed to zero so its wave can start with the first theme.
fn initial_focus() -> Focus {
    Focus {
        navigation_mode:      false,
        focused_module_index: None,
        attention:            Attention::default(),
        hover:                HoverFades::default(),
        hints:                hydebar_core::tooltip::Hints::default(),
        notification_popups:  Vec::new(),
        greeting:             hydebar_core::animation::Spring::new(0.0),
        greeting_line:        String::new(),
        greeting_raised:      Vec::new(),
        greeting_deadline:    None,
        wallpaper_pending:    None,
        bar_layout_pending:   None
    }
}

impl App {
    /// Builds the bar from what the binary has already opened for it.
    ///
    /// Takes its dependencies as one tuple because this is the closure the
    /// widget toolkit boots the application with, and that closure is handed
    /// exactly one value.
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

        let custom = custom_modules(&config);
        let module_context = ModuleContext::new(event_sender, runtime_handle);
        let hyprland_clone = Arc::clone(&hyprland);
        let mut app = Self {
            runtime: Runtime {
                config_path: config_path.clone(),
                logger,
                hyprland,
                config_manager,
                bus_receiver
            },
            motion: initial_motion(config.appearance.clone()),
            theme_cache: hydebar_core::style::hydebar_theme(&config.appearance),
            module_context,
            icons: IconTheme::from_config(&config.icons)
                .with_size(config.appearance.font_size_px()),
            screen: initial_screen(),
            wallpaper_preview: None,
            looks: hydebar_core::modules::desk::looks::Looks::default(),
            history: super::history::History::default(),
            outputs,
            focus: initial_focus(),
            modules: initial_modules(&config, &hyprland_clone, custom, config_path),
            raw_config: None,
            derived_themes: std::cell::RefCell::new(std::collections::HashMap::new()),
            config
        };

        app.register_modules();
        app.arm_birth_animations();

        if app.config.idle_inhibitor.start_activated {
            app.modules.control_center.set_idle_inhibited(true);
        }

        let task = Task::batch([
            task,
            super::super::update::outputs::read_wallpaper(),
            super::super::update::outputs::read_looks()
        ]);

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

        assert!(Arc::ptr_eq(&app.runtime.hyprland, &mock_port));
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
