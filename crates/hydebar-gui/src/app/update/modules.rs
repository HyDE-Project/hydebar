//! Messages forwarded to individual bar modules.

use std::sync::Arc;

use hydebar_core::{
    menu::MenuType,
    modules::{self, tray::TrayMessage},
    services::{ServiceEvent, tray::TrayEvent},
    utils
};
use iced::Task;
use log::error;

use super::super::state::{App, Message};

impl App {
    /// Shows a popup for a notification that has just arrived.
    ///
    /// A notification the user dismissed is taken off the screen at once, so
    /// clearing the centre also clears what is floating above it.
    fn raise_popup(&mut self, message: &modules::notifications::NotificationsMessage) {
        use hydebar_core::{
            notifications_popup::Popup,
            services::{ServiceEvent, notifications::NotificationEvent}
        };

        match message {
            modules::notifications::NotificationsMessage::Event(ServiceEvent::Update(
                NotificationEvent::Received(notification)
            )) => {
                if self.config.notifications.source.hands_to_compositor() {
                    self.hand_to_compositor(notification);
                    return;
                }

                if !self.config.notifications.source.draws_popups() {
                    return;
                }

                self.notification_popups
                    .push(Popup::new(notification, std::time::Instant::now()));
                hydebar_core::notifications_popup::prune(
                    &mut self.notification_popups,
                    std::time::Instant::now()
                );
            }
            modules::notifications::NotificationsMessage::Dismiss(id) => {
                self.notification_popups.retain(|popup| popup.id != *id);
            }
            modules::notifications::NotificationsMessage::ClearAll => {
                self.notification_popups.clear();
            }
            _ => {}
        }
    }

    /// Hands a notification to the compositor, painted with the theme.
    ///
    /// Nothing is drawn by the bar in this mode: the compositor shows its own
    /// notification, which is what a session that wants no extra pieces asked
    /// for.
    fn hand_to_compositor(
        &self,
        notification: &hydebar_core::services::notifications::Notification
    ) {
        use hydebar_core::services::{
            hyprland_notify::{Notice, compositor_color, notify},
            notifications::Urgency
        };

        let notice = match notification.urgency {
            Urgency::Low => Notice::Hint,
            Urgency::Normal => Notice::Info,
            Urgency::Critical => Notice::Error
        };

        let color = compositor_color(self.appearance().primary_color);
        let duration = hydebar_core::notifications_popup::lifetime_for(&notification.urgency);
        let message = if notification.body.is_empty() {
            notification.summary.clone()
        } else {
            format!("{}: {}", notification.summary, notification.body)
        };

        notify(
            notice,
            duration.as_millis() as u32,
            &color,
            self.appearance().font_size_px(),
            &message
        );
    }

    /// Grows or shrinks the notification surface to what the popups need.
    pub(super) fn fit_notification_surface(&mut self) -> Task<Message> {
        let height = hydebar_core::notifications_popup::surface_height(
            &self.notification_popups,
            self.appearance()
        );

        self.outputs.resize_notifications(height)
    }

    /// Handles the messages this module owns.
    pub(super) fn update_modules(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Updates(message) => {
                if let Some(updates_config) = self.config.updates.as_ref() {
                    self.updates
                        .update(message, updates_config, &mut self.outputs, &self.config);
                }
                Task::none()
            }
            Message::OpenLauncher => {
                if let Some(app_launcher_cmd) = self.config.app_launcher_cmd.as_ref() {
                    utils::launcher::execute_command(app_launcher_cmd.to_string());
                }
                Task::none()
            }
            Message::LaunchCommand(command) => {
                utils::launcher::execute_command(command);
                Task::none()
            }
            Message::CustomMenuAction(id, command) => {
                utils::launcher::execute_command(command);
                self.outputs.close_menu(id, &self.config)
            }
            Message::CustomUpdate(name, message) => {
                match self.custom.get_mut(&name) {
                    Some(c) => c.update(message),
                    None => error!("Custom module '{name}' not found")
                };
                Task::none()
            }
            Message::OpenClipboard => {
                if let Some(clipboard_cmd) = self.config.clipboard_cmd.as_ref() {
                    utils::launcher::execute_command(clipboard_cmd.to_string());
                }
                Task::none()
            }
            Message::Workspaces(msg) => {
                self.workspaces.update(msg, &self.config.workspaces);

                Task::none()
            }
            Message::WindowTitle(message) => {
                self.window_title.update(message, &self.config.window_title);
                Task::none()
            }
            Message::SystemInfo(message) => {
                self.system_info.update(message, &self.config.system);
                Task::none()
            }
            Message::KeyboardLayout(message) => {
                self.keyboard_layout.update(message);
                Task::none()
            }
            Message::KeyboardSubmap(message) => {
                self.keyboard_submap.update(message);
                Task::none()
            }
            Message::Tray(msg) => {
                let close_tray = match &msg {
                    TrayMessage::Event(event) => {
                        if let ServiceEvent::Update(TrayEvent::Unregistered(name)) = event.as_ref()
                        {
                            self.outputs
                                .close_all_menu_if(MenuType::Tray(name.clone()), &self.config)
                        } else {
                            Task::none()
                        }
                    }
                    _ => Task::none()
                };

                self.tray.update(msg);
                close_tray
            }
            Message::Clock(message) => {
                self.clock.update(
                    message,
                    &self.config.clock,
                    self.config.appearance.animations.enabled
                );
                Task::none()
            }
            Message::Calendar(message) => {
                self.calendar.update(message);
                Task::none()
            }
            Message::HydeMenu(message) => match self.hyde_menu.update(message) {
                Some((surface, command)) => {
                    hydebar_core::utils::launcher::execute_command(command);

                    self.outputs.close_menu(surface, &self.config)
                }
                None => Task::none()
            },
            Message::Weather(message) => {
                self.weather.update(message);
                Task::none()
            }
            Message::Battery(message) => {
                self.battery.update(message);
                Task::none()
            }
            Message::Privacy(msg) => {
                self.privacy.update(msg);
                Task::none()
            }
            Message::ControlCenter(message) => {
                self.control_center.update(
                    message,
                    &self.config.control_center,
                    &mut self.outputs,
                    &self.config
                );
                Task::none()
            }
            Message::Settings(msg) => {
                let config = Arc::clone(&self.config);

                self.settings.update(msg, &config).map(Message::Settings)
            }
            Message::Themes(msg) => {
                let config = Arc::clone(&self.config);

                self.themes.update(msg, &config).map(Message::Themes)
            }
            Message::Wallpaper(msg) => {
                let config = Arc::clone(&self.config);

                self.wallpaper.update(msg, &config).map(Message::Wallpaper)
            }
            Message::MediaPlayer(msg) => {
                self.media_player.update(msg);
                Task::none()
            }
            Message::ExpirePopups => {
                let before = self.notification_popups.len();

                hydebar_core::notifications_popup::prune(
                    &mut self.notification_popups,
                    std::time::Instant::now()
                );

                if before == self.notification_popups.len() {
                    Task::none()
                } else {
                    self.fit_notification_surface()
                }
            }
            Message::Notifications(msg) => {
                let popups_before = self.notification_popups.len();

                self.raise_popup(&msg);
                self.notifications.update(msg);

                if popups_before == self.notification_popups.len() {
                    Task::none()
                } else {
                    self.fit_notification_surface()
                }
            }
            Message::Screenshot(msg) => {
                self.screenshot.update(msg);
                Task::none()
            }
            _ => Task::none()
        }
    }
}
