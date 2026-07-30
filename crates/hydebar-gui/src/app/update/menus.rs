//! Opening and closing menu surfaces.

use std::time::Duration;

use hydebar_core::{
    config::ModuleName,
    menu::MenuType,
    modules::{self, control_center::brightness::BrightnessMessage},
    services::brightness::BrightnessCommand
};
use iced::Task;

/// Pause between the pointer leaving the sound module or its menu and the
/// menu being taken down, long enough to travel between the two.
const SOUND_HOVER_GRACE: Duration = Duration::from_millis(300);

use super::super::state::{App, Message};

impl App {
    /// Restates the attention from the menu that is open, if one is.
    ///
    /// An open menu outranks the pointer: the user opened it to read it, and
    /// the pointer has to leave the module to reach the menu at all. Closing
    /// the last menu releases the attention rather than handing it back to
    /// whatever the pointer happens to be over, so nothing stays attended by
    /// accident.
    fn attend_the_open_menu(&mut self) {
        let focus = self.outputs.open_menu().map(MenuType::owner);

        self.attention.look_at(focus);
    }

    /// Schedules the check that takes the sound menu down after a leave.
    ///
    /// The pause covers the travel between the sound module and its menu:
    /// closing on the leave itself would take the menu away while the pointer
    /// is still on its way into it.
    fn settle_sound_hover() -> Task<Message> {
        Task::perform(tokio::time::sleep(SOUND_HOVER_GRACE), |()| {
            Message::SoundHoverSettle
        })
    }

    /// Handles the messages this module owns.
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

                if matches!(module, ModuleName::Audio) {
                    self.sound_on_module = entered;

                    if !entered {
                        return Task::batch(vec![
                            self.outputs.hide_tooltip(surface, Some(&module)),
                            Self::settle_sound_hover(),
                        ]);
                    }

                    if self.outputs.open_menu() != Some(&MenuType::Audio)
                        && let Some(info) = &tooltip
                    {
                        let task = self.outputs.toggle_menu(
                            surface,
                            MenuType::Audio,
                            info.anchor,
                            &self.config
                        );

                        self.attend_the_open_menu();

                        return task;
                    }

                    return self.outputs.hide_tooltip(surface, None);
                }

                match tooltip {
                    Some(info) => self.outputs.show_tooltip(surface, module, info),
                    None if entered => self.outputs.hide_tooltip(surface, None),
                    None => self.outputs.hide_tooltip(surface, Some(&module))
                }
            }
            Message::SoundMenuHover(entered) => {
                self.sound_on_menu = entered;

                if entered {
                    Task::none()
                } else {
                    Self::settle_sound_hover()
                }
            }
            Message::SoundSurfaceEntered => {
                self.sound_on_module = false;

                if self.sound_on_menu {
                    Task::none()
                } else {
                    Self::settle_sound_hover()
                }
            }
            Message::SoundHoverSettle => {
                if !self.sound_on_module
                    && !self.sound_on_menu
                    && self.outputs.open_menu() == Some(&MenuType::Audio)
                {
                    let task = self.outputs.close_all_menus(&self.config);
                    self.attend_the_open_menu();

                    task
                } else {
                    Task::none()
                }
            }
            Message::ToggleMenu(menu_type, id, button_ui_ref) => {
                let mut cmd = vec![];
                match &menu_type {
                    MenuType::Updates => {
                        self.updates.is_updates_list_open = false;
                    }
                    MenuType::Tray(name) => {
                        if let Some(_tray) = self
                            .tray
                            .service
                            .as_ref()
                            .and_then(|t| t.iter().find(|t| &t.name == name))
                        {
                            self.tray.submenus.clear();
                        }
                    }
                    MenuType::Themes => {
                        cmd.push(self.themes.load_swatches().map(Message::Themes));
                    }
                    MenuType::ControlCenter => {
                        self.control_center.sub_menu = None;

                        if let Some(brightness) = self.control_center.brightness.as_mut() {
                            use hydebar_core::services::Service;
                            cmd.push(brightness.command(BrightnessCommand::Refresh).map(
                                |event| {
                                    Message::ControlCenter(
                                        modules::control_center::Message::Brightness(
                                            BrightnessMessage::Event(event)
                                        )
                                    )
                                }
                            ));
                        }
                    }
                    _ => {}
                };
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
