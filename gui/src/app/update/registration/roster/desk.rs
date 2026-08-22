//! What the user does with the session: launchers, windows, the canvas.

use hydebar_proto::config::ModuleName;

use super::super::{super::super::state::App, gate::gate};

impl App {
    /// Gates the modules the user works the session through.
    ///
    /// The window list is gated on the canvas as well as on the strip: a
    /// miniature of a workspace is the windows standing on it, so the list is
    /// wanted whenever the canvas can unfold and not only when the strip
    /// carries a taskbar. It is still a gate — a bar with neither the entry
    /// nor the canvas starts nothing.
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
            hosts(ModuleName::Taskbar) || self.config.desk.enabled,
            &mut self.taskbar,
            ctx,
            ()
        );
        gate("desk", self.config.desk.enabled, &mut self.desk, ctx, ());
    }
}
