//! What the bar keeps telling the user: the time, the sky, the machine.

use hydebar_proto::config::ModuleName;

use super::super::{
    super::super::state::App,
    gate::{SYSTEM_INFO_CONSUMERS, gate}
};

impl App {
    /// Gates the modules that keep a reading on the strip.
    pub(super) fn register_readings(&mut self) {
        let ctx = &self.module_context;

        let layout = &self.config.modules;
        let hosts = |name: ModuleName| layout.hosts(&name);

        gate(
            "clock",
            hosts(ModuleName::Clock),
            &mut self.clock,
            ctx,
            &self.config.clock
        );
        gate(
            "weather",
            hosts(ModuleName::Weather)
                || (hosts(ModuleName::Clock) && self.config.clock.show_weather),
            &mut self.weather,
            ctx,
            &self.config.weather
        );
        gate(
            "updates",
            hosts(ModuleName::Updates),
            &mut self.updates,
            ctx,
            self.config.updates.as_ref()
        );
        gate(
            "system-info",
            layout.hosts_any(&SYSTEM_INFO_CONSUMERS),
            &mut self.system_info,
            ctx,
            (&self.config.system, layout.hosts(&ModuleName::SystemInfo))
        );
        gate(
            "keyboard-layout",
            hosts(ModuleName::KeyboardLayout),
            &mut self.keyboard_layout,
            ctx,
            ()
        );
        gate(
            "keyboard-submap",
            hosts(ModuleName::KeyboardSubmap),
            &mut self.keyboard_submap,
            ctx,
            ()
        );
    }
}
