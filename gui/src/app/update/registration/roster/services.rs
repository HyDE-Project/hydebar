//! What listens to the machine: its trays, its bells, its hardware.

use hydebar_proto::config::ModuleName;

use super::super::{
    super::super::state::App,
    gate::{CONTROL_CENTER_CONSUMERS, gate, notifications_hosted}
};

impl App {
    /// Gates the modules that listen to the session on the bar's behalf.
    pub(super) fn register_services(&mut self) {
        let ctx = &self.module_context;

        let layout = &self.config.modules;
        let hosts = |name: ModuleName| layout.hosts(&name);

        gate("tray", hosts(ModuleName::Tray), &mut self.tray, ctx, ());
        gate(
            "privacy",
            hosts(ModuleName::Privacy),
            &mut self.privacy,
            ctx,
            ()
        );
        gate(
            "hardware-services",
            layout.hosts_any(&CONTROL_CENTER_CONSUMERS),
            &mut self.control_center,
            ctx,
            ()
        );
        gate(
            "media-player",
            hosts(ModuleName::MediaPlayer),
            &mut self.media_player,
            ctx,
            ()
        );
        gate(
            "notifications",
            notifications_hosted(&self.config),
            &mut self.notifications,
            ctx,
            ()
        );
        gate(
            "screenshot",
            hosts(ModuleName::Screenshot),
            &mut self.screenshot,
            ctx,
            ()
        );
    }
}
