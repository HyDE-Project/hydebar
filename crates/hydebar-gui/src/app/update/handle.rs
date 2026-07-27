//! Dispatch of a single application message.

use hydebar_core::{
    menu::MenuType,
    modules::{self, OnModulePress, settings::brightness::BrightnessMessage, tray::TrayMessage},
    position_button::ButtonUIRef,
    services::{ServiceEvent, brightness::BrightnessCommand, tray::TrayEvent},
    utils
};
use iced::{Task, event::wayland::OutputEvent};
use log::{debug, error, info, warn};

use super::super::state::{App, Message};
use crate::get_log_spec;

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Frame(now) => {
                let elapsed = self
                    .last_frame
                    .map(|last| now.saturating_duration_since(last))
                    .unwrap_or_default();
                self.last_frame = Some(now);

                let menus_animating = self
                    .outputs
                    .tick_menu_animations(&self.config.appearance.animations, elapsed);
                let theme_animating = self.appearance_transition.advance(elapsed);

                if !menus_animating && !theme_animating {
                    self.last_frame = None;
                }

                Task::none()
            }
            Message::BusFlushed(outcome) => {
                if outcome.had_error() {
                    error!("event bus reported a failure while delivering events");
                }

                if outcome.is_empty() {
                    Task::none()
                } else {
                    let tasks: Vec<_> = outcome
                        .into_events()
                        .into_iter()
                        .filter_map(App::message_from_bus_event)
                        .map(|msg| self.update(msg))
                        .collect();

                    Task::batch(tasks)
                }
            }
            Message::None => Task::none(),
            Message::ConfigChanged(update) => {
                let hydebar_core::config::ConfigApplied {
                    config,
                    impact
                } = update;

                info!("New config applied: {config:?}");
                debug!("Config impact: {impact:?}");

                let mut tasks = Vec::new();

                let outputs_need_sync = impact.outputs_changed
                    || impact.position_changed
                    || self.config.appearance.style != config.appearance.style
                    || self.config.appearance.scale_factor != config.appearance.scale_factor;

                if outputs_need_sync {
                    warn!("Outputs or layout changed, syncing");
                    tasks.push(self.outputs.sync(
                        config.appearance.style,
                        &config.outputs,
                        config.position,
                        &config
                    ));
                }

                if impact.custom_modules_changed {
                    self.update_custom_modules(&config, &impact);
                }

                self.config = config;

                let blend_palette = self.config.appearance.animations.enabled;
                self.appearance_transition
                    .set_target(self.config.appearance.clone(), blend_palette);

                self.register_modules();

                if impact.log_level_changed {
                    self.logger
                        .set_new_spec(get_log_spec(&self.config.log_level));
                }

                Task::batch(tasks)
            }
            Message::ConfigDegraded(degradation) => {
                warn!("Configuration degradation reported: {}", degradation.reason);
                Task::none()
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
                    MenuType::Settings => {
                        self.settings.sub_menu = None;

                        if let Some(brightness) = self.settings.brightness.as_mut() {
                            use hydebar_core::services::Service;
                            cmd.push(brightness.command(BrightnessCommand::Refresh).map(
                                |event| {
                                    Message::Settings(modules::settings::Message::Brightness(
                                        BrightnessMessage::Event(event)
                                    ))
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

                Task::batch(cmd)
            }
            Message::CloseMenu(id) => self.outputs.close_menu(id, &self.config),
            Message::CloseAllMenus => {
                if self.outputs.menu_is_open() {
                    self.outputs.close_all_menus(&self.config)
                } else {
                    Task::none()
                }
            }
            Message::ActivateNavigationMode => {
                if !self.navigation_mode && self.config.keybindings.enabled {
                    info!("Activating navigation mode");
                    self.navigation_mode = true;
                    self.focused_module_index = Some(0);
                }
                Task::none()
            }
            Message::DeactivateNavigationMode => {
                if self.navigation_mode {
                    info!("Deactivating navigation mode");
                    self.navigation_mode = false;
                    self.focused_module_index = None;
                }
                if self.outputs.menu_is_open() {
                    self.outputs.close_all_menus(&self.config)
                } else {
                    Task::none()
                }
            }
            Message::NavigateUp | Message::NavigateDown => {
                if !self.navigation_mode {
                    return Task::none();
                }

                Task::none()
            }
            Message::NavigateLeft => {
                if !self.navigation_mode {
                    return Task::none();
                }

                if let Some(current) = self.focused_module_index
                    && current > 0
                {
                    self.focused_module_index = Some(current - 1);
                    debug!("Navigate left: focus moved to module {}", current - 1);
                }
                Task::none()
            }
            Message::NavigateRight => {
                if !self.navigation_mode {
                    return Task::none();
                }

                if let Some(current) = self.focused_module_index {
                    let all_modules = self.get_all_modules_count();
                    if current + 1 < all_modules {
                        self.focused_module_index = Some(current + 1);
                        debug!("Navigate right: focus moved to module {}", current + 1);
                    }
                }
                Task::none()
            }
            Message::ActivateFocusedModule => {
                if !self.navigation_mode || self.focused_module_index.is_none() {
                    return Task::none();
                }

                let index = self.focused_module_index.unwrap();

                let main_window_id = if let Some(id) = self.outputs.first_main_window_id() {
                    id
                } else {
                    return Task::none();
                };

                if let Some(action) = self.get_module_at_index(index, main_window_id) {
                    match action {
                        OnModulePress::Action(msg) => {
                            info!("Activating module at index {} with action", index);
                            return self.update(*msg);
                        }
                        OnModulePress::ToggleMenu(menu_type) => {
                            info!(
                                "Activating module at index {} - opening menu {:?}",
                                index, menu_type
                            );

                            let center_button_ref = ButtonUIRef {
                                position: iced::Point {
                                    x: 960.0, y: 20.0
                                },
                                viewport: (1920.0, 1080.0)
                            };

                            return self.update(Message::ToggleMenu(
                                menu_type,
                                main_window_id,
                                center_button_ref
                            ));
                        }
                    }
                }

                Task::none()
            }
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
                self.system_info.update(message);
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
                self.clock.update(message);
                Task::none()
            }
            Message::Weather(message) => {
                self.weather.update(message.clone());

                // If clock is configured to show weather, update it too
                if self.config.clock.show_weather
                    && let modules::weather::Message::Update(weather_data) = message
                {
                    self.clock
                        .update(modules::clock::Message::UpdateWeather(weather_data));
                }

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
            Message::Settings(message) => {
                self.settings.update(
                    message,
                    &self.config.settings,
                    &mut self.outputs,
                    &self.config
                );
                Task::none()
            }
            Message::OutputEvent((event, wl_output)) => match event {
                OutputEvent::Created(info) => {
                    info!("Output created: {info:?}");
                    let name = info
                        .as_ref()
                        .and_then(|info| info.name.as_deref())
                        .unwrap_or("");

                    self.outputs.add(
                        self.config.appearance.style,
                        &self.config.outputs,
                        self.config.position,
                        name,
                        wl_output,
                        &self.config
                    )
                }
                OutputEvent::Removed => {
                    info!("Output destroyed");
                    self.outputs.remove(
                        self.config.appearance.style,
                        self.config.position,
                        wl_output,
                        &self.config
                    )
                }
                _ => Task::none()
            },
            Message::MediaPlayer(msg) => {
                self.media_player.update(msg);
                Task::none()
            }
            Message::Notifications(msg) => {
                self.notifications.update(msg);
                Task::none()
            }
            Message::Screenshot(msg) => {
                self.screenshot.update(msg);
                Task::none()
            }
        }
    }
}
