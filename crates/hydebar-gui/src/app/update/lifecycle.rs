//! Frame ticks, bus drains and configuration reloads.

use iced::Task;
use log::{debug, error, info, warn};

use super::super::{
    shutdown,
    state::{App, Message}
};
use crate::get_log_spec;

impl App {
    /// Handles the messages this module owns.
    pub(super) fn update_lifecycle(&mut self, message: Message) -> Task<Message> {
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
                        &config,
                        self.scaled_appearance().height
                    ));
                }

                if impact.custom_modules_changed {
                    self.update_custom_modules(&config, &impact);
                }

                self.config = config;
                let resize = self.refresh_appearance();

                self.register_modules();

                if impact.log_level_changed {
                    self.logger
                        .set_new_spec(get_log_spec(&self.config.log_level));
                }

                tasks.push(resize);

                Task::batch(tasks)
            }
            Message::Shutdown(signal) => {
                info!("shutting down on {signal:?}, removing every surface");
                shutdown::exit_after_flush();

                self.outputs.destroy_all()
            }
            Message::ConfigDegraded(degradation) => {
                warn!("Configuration degradation reported: {}", degradation.reason);
                Task::none()
            }
            _ => Task::none()
        }
    }
}
