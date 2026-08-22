//! Handling of settings menu messages.

use iced::Task;
use tokio::runtime::Handle;

use super::super::{ControlCenter, Message, SubMenu, commands::ControlCenterCommandExt};
use crate::{
    ModuleEventSender, config::ControlCenterModuleConfig, menu::MenuType, outputs::Outputs,
    services::network::NetworkCommand
};

mod audio;
mod bluetooth;
mod brightness;
mod network;
mod password_dialog;
mod upower;

impl ControlCenter {
    pub(crate) fn runtime(&self) -> Option<Handle> {
        self.runtime.clone()
    }

    pub(crate) fn sender(&self) -> Option<ModuleEventSender<Message>> {
        self.sender.clone()
    }

    /// Schedules the release of an activation that outlives `delay`.
    ///
    /// Without a delay the inhibitor stays until it is toggled off,
    /// which is what a configuration naming no timeout asks
    /// for.
    fn arm_idle_release(&mut self, delay: Option<std::time::Duration>) {
        let (Some(delay), Some(runtime), Some(sender)) = (delay, self.runtime(), self.sender())
        else {
            return;
        };

        self.idle_release = Some(runtime.spawn(async move {
            tokio::time::sleep(delay).await;

            sender.send(Message::ReleaseInhibitIdle);
        }));
    }

    /// Answers one message, handing back whatever it asks the shell for.
    ///
    /// The task is a return value because the shell work is not optional: a
    /// menu taken down destroys one layer surface and raises its successor,
    /// and a task dropped where it was made leaves the old surface mapped and
    /// the menu pointing at an identity nothing owns. The keyboard grabs are
    /// the same — asked for and never made.
    #[must_use = "the shell work a menu asks for does not happen unless the task is run"]
    pub fn update(
        &mut self,
        message: Message,
        config: &ControlCenterModuleConfig,
        outputs: &mut Outputs,
        main_config: &crate::config::Config
    ) -> Task<Message> {
        match message {
            Message::ToggleMenu(id, button_ui_ref) => {
                self.sub_menu = None;
                self.password_dialog = None;

                return outputs.toggle_menu::<Message>(
                    id,
                    MenuType::ControlCenter,
                    button_ui_ref,
                    main_config
                );
            }
            Message::Audio(msg) => {
                return self.handle_audio(msg, config, outputs, main_config);
            }
            Message::UPower(msg) => {
                self.handle_upower(msg);
            }
            Message::Network(msg) => {
                return self.handle_network(msg, config, outputs, main_config);
            }
            Message::Bluetooth(msg) => {
                return self.handle_bluetooth(msg, config, outputs, main_config);
            }
            Message::Brightness(msg) => {
                self.handle_brightness(msg);
            }
            Message::ToggleSubMenu(menu_type) => {
                if self.sub_menu == Some(menu_type) {
                    self.sub_menu.take();
                } else {
                    self.sub_menu.replace(menu_type);

                    if menu_type == SubMenu::Wifi {
                        let _spawned = self.spawn_network_command(NetworkCommand::ScanNearByWiFi);
                    }
                }
            }
            Message::ToggleInhibitIdle => {
                let inhibited = self.is_idle_inhibited();
                self.set_idle_inhibited(!inhibited);

                if self.is_idle_inhibited() {
                    self.arm_idle_release(main_config.idle_inhibitor.release_after());
                }
            }
            Message::ReleaseInhibitIdle => {
                self.set_idle_inhibited(false);
            }
            Message::Lock => {
                if let Some(lock_cmd) = &config.lock_cmd {
                    crate::utils::launcher::execute_command(lock_cmd.clone());
                }
            }
            Message::Power(msg) => {
                msg.update();
            }
            Message::PasswordDialog(msg) => {
                return self.handle_password_dialog(msg, outputs, main_config);
            }
        }

        Task::none()
    }
}
