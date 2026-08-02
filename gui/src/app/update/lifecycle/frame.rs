//! One frame tick: advancing every animation the bar keeps.

use iced::Task;
use log::debug;

use super::super::super::state::{App, GREETING_LIFETIME, Message};

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

    /// Advances every animation of the bar by one frame.
    ///
    /// The greeting lets itself out: its deadline is anchored to the first
    /// frame it was alive on, and the frame clock is guaranteed to tick for
    /// as long as it shows.
    ///
    /// The theme is rebuilt only when the palette moved, settling frame
    /// included: frames driven by hovers, menus or the greeting must not pay
    /// a palette derivation for colours that stood still.
    pub(super) fn on_frame(&mut self, now: std::time::Instant) -> Task<Message> {
        self.derived_themes.borrow_mut().clear();

        let elapsed = self
            .last_frame
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or_default();
        self.last_frame = Some(now);

        if self.greeting.target() > 0.0 {
            let deadline = *self
                .greeting_deadline
                .get_or_insert(now + GREETING_LIFETIME);

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
        let sliding = self.relayout.advance(elapsed);
        let greeting_animating = self.greeting.advance(elapsed);
        let values_fading = self.clock.tick_fade(elapsed)
            | self.updates.tick_fade(elapsed)
            | self.keyboard_layout.tick_fade(elapsed)
            | self.keyboard_submap.tick_fade(elapsed)
            | self.battery.tick_fade(elapsed);
        let greeting_tasks = self.greeting_surface_tasks();

        if palette_moved {
            self.rebuild_theme();
        }

        if !menus_animating
            && !theme_animating
            && !hover_animating
            && !entering
            && !sliding
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
}
