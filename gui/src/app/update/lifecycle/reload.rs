//! Adoption of a reloaded configuration.

use iced::Task;
use log::{debug, info, warn};

use super::super::super::state::{App, Message};
use crate::get_log_spec;

/// Builds the notice announcing `source`, for the bar to paint itself.
fn announcement(
    source: hydebar_proto::config::NotificationSource
) -> hydebar_core::services::notifications::Notification {
    hydebar_core::services::notifications::Notification {
        id:             0,
        app_name:       "hydebar".to_owned(),
        icon:           String::new(),
        summary:        "Notifications".to_owned(),
        body:           format!("now shown by {}", source.label()),
        urgency:        hydebar_core::services::notifications::Urgency::Normal,
        timestamp:      std::time::SystemTime::now(),
        actions:        Vec::new(),
        expire_timeout: -1
    }
}

impl App {
    /// Adopts a configuration the watcher reloaded.
    ///
    /// A theme switch reloads the file several times and most of those
    /// reloads carry exactly what the bar already runs on. The raw text is
    /// compared before adoption on purpose: adoption clones the whole
    /// configuration and may ask the compositor four questions, and none of
    /// that is owed to a reload that changed nothing. The blur restatement
    /// still goes out — the compositor may have wiped the rules regardless of
    /// what the file says.
    ///
    /// A changed notification source is announced here rather than where the
    /// choice was made: the notice has to be painted the new way, and until
    /// the reload has landed the bar would still paint it the old one — which
    /// is exactly the question the notice answers. The bar's own popups are
    /// drawn directly rather than sent through the bus: the bar only takes
    /// the bus name once the new subscription starts, so a notice sent this
    /// instant would reach whoever still held it, or nobody at all. The
    /// popup surface is refitted as well — it is a strip whose height is
    /// whatever its popups need, and without this it stays as tall as it was
    /// when empty and clips the notice away entirely.
    pub(super) fn on_config_changed(
        &mut self,
        update: hydebar_core::config::ConfigApplied
    ) -> Task<Message> {
        let hydebar_core::config::ConfigApplied {
            config,
            impact
        } = update;

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

        let source_changed = self.config.notifications.source != config.notifications.source;

        info!("New config applied: {config:?}");
        debug!("Config impact: {impact:?}");

        let mut tasks = Vec::new();

        if source_changed {
            if config.notifications.source.draws_popups() {
                self.notification_popups
                    .push(hydebar_core::notifications_popup::Popup::new(
                        &announcement(config.notifications.source),
                        std::time::Instant::now()
                    ));
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

        if impact.desk_changed {
            let unfold = self.unfold_desk();
            tasks.push(unfold);
        }

        if (impact.layout_changed || impact.custom_modules_changed)
            && self.config.appearance.animations.enabled
        {
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
}
