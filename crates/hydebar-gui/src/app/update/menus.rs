//! Opening and closing menu surfaces.

use hydebar_core::{
    menu::MenuType,
    modules::{self, control_center::SubMenu}
};
use iced::Task;

use super::super::state::{App, Message};

impl App {
    /// Restates the attention from the menu that is open, if one is.
    ///
    /// An open menu outranks the pointer: the user opened it to read it, and
    /// the pointer has to leave the module to reach the menu at all. Closing
    /// the last menu releases the attention rather than handing it back to
    /// whatever the pointer happens to be over, so nothing stays attended by
    /// accident.
    pub(super) fn attend_the_open_menu(&mut self) {
        let focus = self.outputs.open_menu().map(MenuType::owner);

        self.attention.look_at(focus);
        self.poll_attended_now();
    }

    /// Executes what the tooltip lifecycle asked for, if anything.
    pub(super) fn run_hint_command(
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

    /// Handles the messages this module owns.
    #[expect(
        clippy::too_many_lines,
        reason = "one match arm per menu message, read as a single dispatch table"
    )]
    pub(super) fn update_menus(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ModuleHover {
                surface,
                module,
                entered,
                tooltip
            } => {
                self.attention.follow_pointer(
                    module.clone(),
                    entered,
                    self.outputs.menu_is_open()
                );
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
            Message::ToggleMenu(menu_type, id, button_ui_ref) => {
                self.hints.dismiss();

                let mut cmd = vec![];
                match &menu_type {
                    MenuType::Updates => {
                        self.updates.collapse();
                    }
                    MenuType::Tray(name) => {
                        if self
                            .tray
                            .service
                            .as_ref()
                            .is_some_and(|t| t.iter().any(|t| &t.name == name))
                        {
                            self.tray.collapse_submenus();
                        }
                    }
                    MenuType::Wallpaper => {
                        if self.outputs.open_menu() != Some(&MenuType::Wallpaper) {
                            cmd.push(self.wallpaper.load_entries().map(Message::Wallpaper));

                            if self.wallpaper.is_empty() {
                                self.wallpaper_pending = Some((id, button_ui_ref));
                                self.attend_the_open_menu();

                                return Task::batch(cmd);
                            }
                        }
                    }
                    MenuType::BarLayout => {
                        if self.outputs.open_menu() != Some(&MenuType::BarLayout) {
                            cmd.push(self.bar_layout.load_entries().map(Message::BarLayout));

                            if self.bar_layout.is_empty() {
                                self.bar_layout_pending = Some((id, button_ui_ref));
                                self.attend_the_open_menu();

                                return Task::batch(cmd);
                            }
                        }
                    }
                    MenuType::Themes => {
                        if self.outputs.open_menu() != Some(&MenuType::Themes) {
                            cmd.push(self.themes.load_swatches().map(Message::Themes));
                            cmd.push(self.themes.load_catalogue().map(Message::Themes));
                        }
                    }
                    MenuType::Audio => {
                        self.control_center.open_audio_menu();
                    }
                    MenuType::HydeMenu => {
                        cmd.push(self.hyde_menu.reload().map(Message::HydeMenu));
                    }
                    MenuType::Network => {
                        if self.outputs.open_menu() != Some(&MenuType::Network) {
                            self.control_center.close_submenu();
                            self.control_center.update(
                                modules::control_center::Message::ToggleSubMenu(SubMenu::Wifi),
                                &self.config.control_center,
                                &mut self.outputs,
                                &self.config
                            );
                        }
                    }
                    MenuType::Bluetooth => {
                        self.control_center.open_bluetooth_menu();
                    }
                    MenuType::ControlCenter => {
                        self.control_center.close_submenu();
                        cmd.push(
                            self.control_center
                                .refresh_brightness()
                                .map(Message::ControlCenter)
                        );
                    }
                    _ => {}
                }
                cmd.push(
                    self.outputs
                        .toggle_menu(id, menu_type, button_ui_ref, &self.config)
                );

                self.attend_the_open_menu();

                Task::batch(cmd)
            }
            Message::BarPressed => {
                self.outputs.arm_menu_dismissal();

                Task::none()
            }
            Message::BarReleased => {
                let task = self.outputs.dismiss_armed_menus(&self.config);
                self.attend_the_open_menu();

                task
            }
            Message::CloseMenu(id) => {
                let task = self.outputs.close_menu(id, &self.config);
                self.attend_the_open_menu();

                task
            }
            Message::CloseAllMenus => {
                if self.outputs.menu_is_open() {
                    let task = self.outputs.close_all_menus(&self.config);
                    self.attend_the_open_menu();

                    task
                } else {
                    Task::none()
                }
            }
            _ => Task::none()
        }
    }
}
