//! Wiring of modules to the event bus.

use std::collections::HashMap;

use hydebar_core::{
    config::ConfigImpact,
    modules::{self, custom_module::Custom}
};
use hydebar_proto::config::{Config, ModuleName};
use log::error;

use super::super::state::{App, Message};

impl App {
    pub(crate) fn register_modules(&mut self) {
        let ctx = &self.module_context;
        let register = |name: &str, result: Result<(), modules::ModuleError>| {
            if let Err(err) = result {
                error!("failed to register {name} module: {err}");
            }
        };

        register(
            "app-launcher",
            modules::Module::<Message>::register(&mut self.app_launcher, ctx, ())
        ); // uses optional config at view time
        register(
            "clipboard",
            modules::Module::<Message>::register(&mut self.clipboard, ctx, ())
        );
        self.clock.register(ctx, &self.config.clock);
        self.weather.register(ctx);
        register(
            "updates",
            modules::Module::<Message>::register(
                &mut self.updates,
                ctx,
                self.config.updates.as_ref()
            )
        );
        register(
            "workspaces",
            modules::Module::<Message>::register(
                &mut self.workspaces,
                ctx,
                &self.config.workspaces
            )
        );
        register(
            "window-title",
            modules::Module::<Message>::register(&mut self.window_title, ctx, ())
        );
        register(
            "system-info",
            modules::Module::<Message>::register(&mut self.system_info, ctx, ())
        );
        register(
            "keyboard-layout",
            modules::Module::<Message>::register(&mut self.keyboard_layout, ctx, ())
        );
        register(
            "keyboard-submap",
            modules::Module::<Message>::register(&mut self.keyboard_submap, ctx, ())
        );
        register(
            "tray",
            modules::Module::<Message>::register(&mut self.tray, ctx, ())
        );
        self.battery.register(ctx);
        register(
            "privacy",
            modules::Module::<Message>::register(&mut self.privacy, ctx, ())
        );
        register(
            "settings",
            modules::Module::<Message>::register(&mut self.control_center, ctx, ())
        );
        register(
            "media-player",
            modules::Module::<Message>::register(&mut self.media_player, ctx, ())
        );
        register(
            "notifications",
            modules::Module::<Message>::register(&mut self.notifications, ctx, ())
        );
        register(
            "screenshot",
            modules::Module::<Message>::register(&mut self.screenshot, ctx, ())
        );
        register(
            "idle-inhibitor",
            modules::Module::<Message>::register(&mut self.idle_inhibitor, ctx, ())
        );

        for definition in &self.config.custom_modules {
            match self.custom.get_mut(&definition.name) {
                Some(module) => {
                    if let Err(err) =
                        modules::Module::<Message>::register(module, ctx, Some(definition))
                    {
                        error!(
                            "failed to register custom module '{}': {err}",
                            definition.name
                        );
                    }
                }
                None => error!(
                    "custom module '{}' missing runtime state entry during registration",
                    definition.name
                )
            }
        }

        for (name, module) in self.custom.iter_mut() {
            if !self
                .config
                .custom_modules
                .iter()
                .any(|definition| definition.name == *name)
                && let Err(err) = modules::Module::<Message>::register(module, ctx, None)
            {
                error!("failed to clear registration for custom module '{name}': {err}");
            }
        }
    }

    pub(super) fn update_custom_modules(&mut self, config: &Config, impact: &ConfigImpact) {
        let mut state = HashMap::with_capacity(config.custom_modules.len());

        for module in &config.custom_modules {
            let module_name = module.name.clone();
            let module_key = ModuleName::Custom(module_name.clone());

            let entry = if impact.affects_module(&module_key) {
                Custom::default()
            } else {
                self.custom.remove(module_name.as_str()).unwrap_or_default()
            };

            state.insert(module_name, entry);
        }

        self.custom = state;
    }
}
