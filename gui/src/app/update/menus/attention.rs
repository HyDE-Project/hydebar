//! Attention and tooltip lifecycle of the module under the pointer.

use hydebar_core::{config::ModuleName, menu::MenuType, tooltip::TooltipInfo};
use iced::{SurfaceId as Id, Task};

use super::super::super::state::{App, Message};

impl App {
    /// Restates the attention from the menu that is open, if one is.
    ///
    /// An open menu outranks the pointer: the user opened it to read it, and
    /// the pointer has to leave the module to reach the menu at all. Closing
    /// the last menu releases the attention rather than handing it back to
    /// whatever the pointer happens to be over, so nothing stays attended by
    /// accident.
    pub(crate) fn attend_the_open_menu(&mut self) {
        let focus = self.outputs.open_menu().map(MenuType::owner);

        self.attention.look_at(focus);
        self.poll_attended_now();
    }

    /// Executes what the tooltip lifecycle asked for, if anything.
    pub(crate) fn run_hint_command(
        &mut self,
        command: Option<hydebar_core::tooltip::HintCommand>
    ) -> Task<Message> {
        use hydebar_core::tooltip::HintCommand;

        match command {
            Some(HintCommand::Show {
                surface,
                module,
                info
            }) => self.outputs.show_tooltip(surface, module, info),
            Some(HintCommand::Hide {
                surface,
                owner
            }) => self.outputs.hide_tooltip(surface, owner.as_ref()),
            None => Task::none()
        }
    }

    /// Follows the pointer entering or leaving a module of the bar.
    pub(super) fn on_module_hover(
        &mut self,
        surface: Id,
        module: ModuleName,
        entered: bool,
        tooltip: Option<TooltipInfo>
    ) -> Task<Message> {
        self.attention
            .follow_pointer(module.clone(), entered, self.outputs.menu_is_open());
        self.poll_attended_now();

        let animations = &self.config.appearance.animations;
        let response = std::time::Duration::from_millis(animations.hover_duration_ms);

        if entered {
            self.hover
                .leave_others(&module, animations.enabled, response);
        }

        self.hover
            .point(module.clone(), entered, animations.enabled, response);

        let command = self.hints.observe(
            surface,
            module,
            entered,
            tooltip,
            std::time::Instant::now(),
            animations.enabled
        );

        self.run_hint_command(command)
    }
}
