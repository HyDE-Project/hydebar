//! Frame ticks, bus drains and configuration reloads.

use iced::Task;
use log::{debug, info, warn};

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
    /// Raises the menu surfaces the greeting borrows, or hands them back.
    ///
    /// The greeting lives mid-screen on the menu surfaces, which idle on the
    /// background layer; they are held on the overlay for exactly as long as
    /// the greeting is present. Each surface is raised exactly once — the
    /// real surfaces may only be created a few frames into the bar's life,
    /// so newcomers are checked for every frame, but a surface already
    /// raised costs no further compositor request. The release fires once
    /// and never touches a surface a menu has meanwhile opened on.
    fn greeting_surface_tasks(&mut self) -> Task<Message> {
        let visible = self.greeting.value() > 0.004 || self.greeting.is_animating();

        if visible {
            let newcomers: Vec<_> = self
                .outputs
                .menu_surfaces()
                .into_iter()
                .filter(|(id, _)| !self.greeting_raised.contains(id))
                .map(|(id, _)| id)
                .collect();

            self.greeting_raised.extend(newcomers.iter().copied());

            return Task::batch(
                newcomers
                    .into_iter()
                    .map(|id| iced::set_layer(id, iced::Layer::Overlay))
            );
        }

        if !self.greeting_raised.is_empty() {
            self.greeting_raised.clear();

            return Task::batch(
                self.outputs
                    .menu_surfaces()
                    .into_iter()
                    .filter(|(_, open)| !open)
                    .map(|(id, _)| iced::set_layer(id, iced::Layer::Background))
            );
        }

        Task::none()
    }

    /// Handles the messages this module owns.
    #[expect(
        clippy::too_many_lines,
        reason = "one match arm per lifecycle message, read as a single dispatch table"
    )]
    pub(super) fn update_lifecycle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Frame(now) => {
                self.derived_themes.borrow_mut().clear();

                let elapsed = self
                    .last_frame
                    .map(|last| now.saturating_duration_since(last))
                    .unwrap_or_default();
                self.last_frame = Some(now);

                // the greeting lets itself out: its deadline is anchored to
                // the first frame it was alive on, and the frame clock is
                // guaranteed to tick for as long as it shows
                if self.greeting.target() > 0.0 {
                    let deadline = *self
                        .greeting_deadline
                        .get_or_insert(now + super::super::state::GREETING_LIFETIME);

                    if now >= deadline {
                        debug!("the greeting's three seconds are up, letting it out");
                        self.greeting.set_response(hydebar_core::animation::GENTLE);
                        self.greeting.set_target(0.0);
                    }
                }

                let animated = self.config.appearance.animations.enabled;
                let served = self.hints.served(now, animated);
                let (hints_fading, landed) = self.hints.advance(elapsed);
                let tooltip_tasks =
                    Task::batch([self.run_hint_command(served), self.run_hint_command(landed)]);

                let popups_before = self.notification_popups.len();
                hydebar_core::notifications_popup::prune(&mut self.notification_popups, now);
                let popups_changed = popups_before != self.notification_popups.len();

                let (menus_animating, menu_tasks) = self
                    .outputs
                    .tick_menu_animations(&self.config.appearance.animations, elapsed);
                let (theme_animating, palette_moved) =
                    self.appearance_transition.advance_reporting(elapsed);
                let hover_animating = self.hover.advance(elapsed);
                let entering = self.entrance.advance(elapsed);
                let _sliding = self.relayout.advance(elapsed);
                let greeting_animating = self.greeting.advance(elapsed);
                let values_fading = self.clock.tick_fade(elapsed)
                    | self.updates.tick_fade(elapsed)
                    | self.keyboard_layout.tick_fade(elapsed)
                    | self.keyboard_submap.tick_fade(elapsed)
                    | self.battery.tick_fade(elapsed);
                let greeting_tasks = self.greeting_surface_tasks();

                // rebuilt only when the palette moved, settling frame
                // included: frames driven by hovers, menus or the greeting
                // must not pay a palette derivation for colours that stood
                // still
                if palette_moved {
                    self.rebuild_theme();
                }

                if !menus_animating
                    && !theme_animating
                    && !hover_animating
                    && !entering
                    && !greeting_animating
                    && !hints_fading
                    && !values_fading
                {
                    self.last_frame = None;
                }

                if popups_changed {
                    Task::batch([
                        menu_tasks,
                        greeting_tasks,
                        tooltip_tasks,
                        self.fit_notification_surface()
                    ])
                } else {
                    Task::batch([menu_tasks, greeting_tasks, tooltip_tasks])
                }
            }
            Message::BusFlushed(outcome) => {
                if outcome.is_empty() {
                    Task::none()
                } else {
                    let tasks: Vec<_> = outcome
                        .into_events()
                        .into_iter()
                        .map(Self::message_from_bus_event)
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
            Message::ConfigChanged(update) => {
                let hydebar_core::config::ConfigApplied {
                    config,
                    impact
                } = update;

                // A theme switch reloads the file several times and most of
                // those reloads carry exactly what the bar already runs on.
                // The raw text is compared before adoption on purpose:
                // adoption clones the whole configuration and may ask the
                // compositor four questions, and none of that is owed to a
                // reload that changed nothing. The blur restatement still
                // goes out — the compositor may have wiped the rules
                // regardless of what the file says.
                let raw_unchanged = self
                    .raw_config
                    .as_ref()
                    .is_some_and(|raw| std::sync::Arc::ptr_eq(raw, &config) || raw == &config);

                if raw_unchanged {
                    debug!("config reload carries no change");
                    hydebar_core::outputs::restate_blur();

                    return Task::none();
                }

                self.raw_config = Some(std::sync::Arc::clone(&config));

                let config = self.adopted(&config);

                if self.config == config {
                    debug!("config reload settles to what already runs");
                    hydebar_core::outputs::restate_blur();

                    return Task::none();
                }

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

                #[expect(
                    clippy::float_cmp,
                    reason = "identity check on a value copied verbatim"
                )]
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
                        self.config.appearance.scale_factor,
                        self.config.appearance.height
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

                if impact.layout_changed && self.config.appearance.animations.enabled {
                    self.flip.borrow_mut().depart();
                    self.relayout = hydebar_core::animation::Spring::new(0.0)
                        .with_response(hydebar_core::animation::STANDARD);
                    self.relayout.set_target(1.0);
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
                shutdown::exit_backstop();

                self.outputs
                    .destroy_all()
                    .chain(Task::done(Message::SurfacesRemoved))
            }
            Message::SurfacesRemoved => shutdown::exit_now(),
            Message::ConfigDegraded(degradation) => {
                warn!("Configuration degradation reported: {}", degradation.reason);
                Task::none()
            }
            _ => Task::none()
        }
    }
}
