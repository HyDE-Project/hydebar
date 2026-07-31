//! Frame ticks, bus drains and configuration reloads.

use iced::Task;
use log::{debug, error, info, warn};

use super::super::{
    shutdown,
    state::{App, Message}
};
use crate::get_log_spec;

/// Builds the notice announcing `source`, for the bar to paint itself.
fn announcement(
    source: hydebar_proto::config::NotificationSource
) -> hydebar_core::services::notifications::Notification {
    hydebar_core::services::notifications::Notification {
        id:        0,
        app_name:  "hydebar".to_owned(),
        icon:      String::new(),
        summary:   "Notifications".to_owned(),
        body:      format!("now shown by {}", source.label()),
        urgency:   hydebar_core::services::notifications::Urgency::Normal,
        timestamp: std::time::SystemTime::now(),
        actions:   Vec::new()
    }
}

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

                let popups_before = self.notification_popups.len();
                hydebar_core::notifications_popup::prune(&mut self.notification_popups, now);
                let popups_changed = popups_before != self.notification_popups.len();

                let (menus_animating, menu_tasks) = self
                    .outputs
                    .tick_menu_animations(&self.config.appearance.animations, elapsed);
                let theme_animating = self.appearance_transition.advance(elapsed);
                let hover_animating = self.hover.advance(elapsed);
                let entering = self.entrance.advance(elapsed);

                // rebuilt on the settling frame as well: the last advance
                // lands exactly on the target after reporting it stopped
                self.rebuild_theme();

                if !menus_animating && !theme_animating && !hover_animating && !entering {
                    self.last_frame = None;
                }

                if popups_changed {
                    Task::batch([menu_tasks, self.fit_notification_surface()])
                } else {
                    menu_tasks
                }
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
            Message::PollAtRest => {
                let now = std::time::Instant::now();

                for module in self.attention.due_at_rest(now) {
                    self.poll_module(&module);
                }

                Task::none()
            }
            Message::PollAttended => {
                let now = std::time::Instant::now();

                if let Some(module) = self.attention.due_attended(now) {
                    self.poll_module(&module);
                }

                Task::none()
            }
            Message::None => Task::none(),
            Message::ConfigChanged(update) => {
                let hydebar_core::config::ConfigApplied {
                    config,
                    impact
                } = update;

                let config = self.adopted(config);
                let source_changed =
                    self.config.notifications.source != config.notifications.source;

                info!("New config applied: {config:?}");
                debug!("Config impact: {impact:?}");

                let mut tasks = Vec::new();

                if source_changed {
                    // Announced here rather than where the choice was made: the
                    // notice has to be painted the new way, and until the
                    // reload has landed the bar would still paint it the old
                    // one — which is exactly the question the notice answers.
                    //
                    // The bar's own popups are drawn directly rather than sent
                    // through the bus: the bar only takes the bus name once the
                    // new subscription starts, so a notice sent this instant
                    // would reach whoever still held it, or nobody at all.
                    if config.notifications.source.draws_popups() {
                        self.notification_popups.push(
                            hydebar_core::notifications_popup::Popup::new(
                                &announcement(config.notifications.source),
                                std::time::Instant::now()
                            )
                        );
                        // The surface is a strip whose height is whatever its
                        // popups need; without this it stays as tall as it was
                        // when empty and clips the notice away entirely.
                        tasks.push(self.fit_notification_surface());
                    } else {
                        hydebar_core::modules::settings::announce_source(
                            config.notifications.source,
                            &config
                        );
                    }
                }

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
                        self.scaled_appearance().scale_factor,
                        self.scaled_appearance().height
                    ));
                }

                if impact.custom_modules_changed {
                    self.update_custom_modules(&config, &impact);
                }

                self.config = config;
                self.themes.refresh();
                hydebar_core::outputs::restate_blur();
                let resize = self.refresh_appearance();

                if impact.moves_module_registration() {
                    self.register_modules();
                }

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
