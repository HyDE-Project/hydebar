//! What the user does with the session: launchers, windows, the canvas.

use hydebar_proto::config::ModuleName;

use super::super::{super::super::state::App, gate::gate};

impl App {
    /// Gates the modules the user works the session through.
    pub(super) fn register_desk_modules(&mut self) {
        let ctx = &self.module_context;

        let layout = &self.config.modules;
        let hosts = |name: ModuleName| layout.hosts(&name);

        gate(
            "app-launcher",
            hosts(ModuleName::AppLauncher),
            &mut self.app_launcher,
            ctx,
            ()
        );
        gate(
            "clipboard",
            hosts(ModuleName::Clipboard),
            &mut self.clipboard,
            ctx,
            ()
        );
        gate(
            "hyde-menu",
            hosts(ModuleName::HydeMenu),
            &mut self.hyde_menu,
            ctx,
            ()
        );
        gate(
            "workspaces",
            hosts(ModuleName::Workspaces),
            &mut self.workspaces,
            ctx,
            &self.config.workspaces
        );
        gate(
            "window-title",
            hosts(ModuleName::WindowTitle),
            &mut self.window_title,
            ctx,
            ()
        );
        gate(
            "taskbar",
            hosts(ModuleName::Taskbar),
            &mut self.taskbar,
            ctx,
            ()
        );
        gate("desk", self.config.desk.enabled, &mut self.desk, ctx, ());
    }
}
