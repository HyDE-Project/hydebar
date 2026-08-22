//! Handling of the Wi-Fi password dialog: typing, confirming and
//! cancelling.

use iced::Task;

use super::super::super::{ControlCenter, Message, commands::ControlCenterCommandExt};
use crate::{outputs::Outputs, password_dialog, services::network::NetworkCommand};

impl ControlCenter {
    #[must_use = "the shell work a menu asks for does not happen unless the task is run"]
    pub(super) fn handle_password_dialog(
        &mut self,
        msg: password_dialog::Message,
        outputs: &Outputs,
        main_config: &crate::config::Config
    ) -> Task<Message> {
        match msg {
            password_dialog::Message::PasswordChanged(password) => {
                if let Some((_, current_password)) = &mut self.password_dialog {
                    *current_password = password;
                }
            }
            password_dialog::Message::DialogConfirmed(id) => {
                if let Some((ssid, password)) = self.password_dialog.take()
                    && let Some(network) = self.network.as_ref()
                    && let Some(access_point) = network
                        .wireless_access_points
                        .iter()
                        .find(|ap| ap.ssid == ssid)
                        .cloned()
                {
                    self.spawn_network_command(NetworkCommand::SelectAccessPoint((
                        access_point,
                        Some(password)
                    )));
                }

                return outputs.release_keyboard::<Message>(id, main_config.menu_keyboard_focus);
            }
            password_dialog::Message::DialogCancelled(id) => {
                self.password_dialog = None;

                return outputs.release_keyboard::<Message>(id, main_config.menu_keyboard_focus);
            }
        }

        Task::none()
    }
}
