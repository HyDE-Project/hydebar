//! Attention and tooltip lifecycle of the module under the pointer.

use hydebar_core::{
    config::ModuleName, menu::MenuType, position_button::ButtonUIRef, tooltip::TooltipInfo
};
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

        self.focus.attention.look_at(focus);
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

    fn hover_menu_target(module: &ModuleName) -> Option<MenuType> {
        match module {
            ModuleName::Updates => Some(MenuType::Updates),
            ModuleName::ControlCenter | ModuleName::Battery | ModuleName::Brightness => {
                Some(MenuType::ControlCenter)
            }
            ModuleName::Audio => Some(MenuType::Audio),
            ModuleName::Network => Some(MenuType::Network),
            ModuleName::Bluetooth => Some(MenuType::Bluetooth),
            ModuleName::PowerProfile => Some(MenuType::PowerProfile),
            ModuleName::Settings => Some(MenuType::Settings),
            ModuleName::HydeMenu => Some(MenuType::HydeMenu),
            ModuleName::Themes => Some(MenuType::Themes),
            ModuleName::MediaPlayer => Some(MenuType::MediaPlayer),
            ModuleName::SystemInfo => Some(MenuType::SystemInfo),
            ModuleName::Cpu => Some(MenuType::Cpu),
            ModuleName::Memory => Some(MenuType::Memory),
            ModuleName::Wallpaper => Some(MenuType::Wallpaper),
            ModuleName::BarLayout => Some(MenuType::BarLayout),
            ModuleName::CpuTemp => Some(MenuType::CpuTemp),
            ModuleName::GpuTemp => Some(MenuType::Gpu),
            ModuleName::Notifications => Some(MenuType::Notifications),
            ModuleName::Screenshot => Some(MenuType::Screenshot),
            ModuleName::Clock => Some(MenuType::Calendar),
            ModuleName::Custom(name) => Some(MenuType::Custom(name.clone())),
            ModuleName::AppLauncher
            | ModuleName::Clipboard
            | ModuleName::Workspaces
            | ModuleName::WindowTitle
            | ModuleName::KeyboardLayout
            | ModuleName::KeyboardSubmap
            | ModuleName::Tray
            | ModuleName::Taskbar
            | ModuleName::Privacy
            | ModuleName::KeybindHint
            | ModuleName::NightLight
            | ModuleName::GameMode
            | ModuleName::Weather
            | ModuleName::IdleInhibitor => None
        }
    }

    /// Follows the pointer entering or leaving a module of the bar.
    pub(super) fn on_module_hover(
        &mut self,
        surface: Id,
        module: ModuleName,
        entered: bool,
        tooltip: Option<TooltipInfo>,
        anchor: Option<ButtonUIRef>
    ) -> Task<Message> {
        let menu_switch = if entered && self.outputs.menu_is_open() {
            match (Self::hover_menu_target(&module), anchor) {
                (Some(target), Some(button_ui_ref))
                    if self.outputs.open_menu() != Some(&target) =>
                {
                    Some(self.on_toggle_menu(target, surface, button_ui_ref))
                }
                _ => None
            }
        } else {
            None
        };

        self.focus
            .attention
            .follow_pointer(module.clone(), entered, self.outputs.menu_is_open());
        self.poll_attended_now();

        let animations = &self.config.appearance.animations;
        let response = std::time::Duration::from_millis(animations.hover_duration_ms);

        if entered {
            self.focus
                .hover
                .leave_others(&module, animations.enabled, response);
        }

        self.focus
            .hover
            .point(module.clone(), entered, animations.enabled, response);

        let command = self.focus.hints.observe(
            surface,
            module,
            entered,
            tooltip,
            std::time::Instant::now(),
            animations.enabled
        );

        let hint_task = self.run_hint_command(command);

        match menu_switch {
            Some(task) => Task::batch([task, hint_task]),
            None => hint_task
        }
    }
}
