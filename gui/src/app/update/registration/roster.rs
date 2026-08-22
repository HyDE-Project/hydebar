//! The roster of modules wired to the event bus on every reload.
//!
//! Registration is what starts pollers, D-Bus listeners and scheduled
//! commands, and none of them has a reader unless the module is rendered
//! somewhere. Gating on placement is what keeps an idle bar idle: a session
//! showing only a clock and workspaces pays for a clock and workspaces rather
//! than for every module the binary happens to ship.
//!
//! The roster is one list, read in three parts by what a module is: [`desk`]
//! is what the user does with the session, [`readings`] is what the bar keeps
//! telling them, and [`services`] is what listens to the machine. The custom
//! modules are a roster of their own, read from the configuration rather than
//! written down here, so they are gated at the end.

mod desk;
mod readings;
mod services;

use hydebar_core::modules;
use hydebar_proto::config::ModuleName;
use log::error;

use super::{
    super::super::state::{App, Message},
    gate::gate
};

impl App {
    /// Registers the background work of every module the layout draws, and
    /// releases it for every module it does not.
    ///
    /// Called again after every configuration reload, so a module added to or
    /// removed from the layout starts and stops with it.
    pub(crate) fn register_modules(&mut self) {
        self.register_desk_modules();
        self.register_readings();
        self.register_services();
        self.register_custom_modules();

        self.place_pollable_modules();
    }

    /// Gates the modules the configuration declares by hand.
    ///
    /// A definition without runtime state is a bug in the loading, not in the
    /// layout, so it is logged rather than gated; one whose definition is gone
    /// is released whatever the layout says.
    fn register_custom_modules(&mut self) {
        let ctx = &self.module_context;
        let layout = &self.config.modules;

        for definition in &self.config.custom_modules {
            let placed = layout.hosts(&ModuleName::Custom(definition.name.clone()));

            match self.custom.get_mut(&definition.name) {
                Some(module) => gate(&definition.name, placed, module, ctx, definition),
                None => error!(
                    "custom module '{}' missing runtime state entry during registration",
                    definition.name
                )
            }
        }

        for (name, module) in &mut self.custom {
            if !self
                .config
                .custom_modules
                .iter()
                .any(|definition| definition.name == *name)
            {
                modules::Module::<Message>::deregister(module);
            }
        }
    }
}
