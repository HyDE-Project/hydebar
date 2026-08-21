//! The unfolding of the desk, screen by screen.

use std::time::Duration;

/// The briefest the whole unfolding is allowed to be.
const SHORTEST_UNFOLDING: Duration = Duration::from_millis(600);

/// The longest the whole unfolding is allowed to be.
const LONGEST_UNFOLDING: Duration = Duration::from_millis(2400);

/// How far into the unfolding the strip's background has gone out.
///
/// Early: the strip is meant to be out of the way while the islands come
/// down, not fading behind them for the whole journey.
const WASH_GOES_OUT_BY: f32 = 0.22;
use iced::{Task, set_input_region};

use super::super::super::state::{App, Message};

impl App {
    /// Sends every screen's canvas towards unfolded or folded, as it stands.
    ///
    /// One spring apiece: the screens answer for themselves, so a monitor
    /// folding back under a window that just mapped must not drag the one
    /// still unfolding over a cleared workspace with it.
    ///
    /// Called whenever the answer can have moved — a fresh reading of the
    /// screens, a configuration that switched the desk off, a monitor that
    /// arrived — because a spring nobody points at never travels, and the
    /// canvas is drawn out of exactly that travel.
    ///
    /// The seats the strip's islands rest at are frozen the moment the canvas
    /// takes the screen: every block then travels from the seat it held on
    /// the strip to the one it takes on the canvas, which is what makes the
    /// islands leave the bar rather than vanish from it.
    ///
    /// The canvas travels out and snaps back. Unfolding happens on a screen
    /// nobody is using, so it can take its time; folding happens because a
    /// window has just taken the screen, and that window is on it already —
    /// a strip still travelling home while the window waits for it reads as
    /// the bar lagging behind the compositor, so the way back is not
    /// travelled at all.
    ///
    /// The returned task hands the canvas its pointer input, or takes it
    /// away: the surface spans the whole screen, so one left taking presses
    /// while it is folded away would swallow every press meant for the
    /// desktop under it.
    pub(crate) fn unfold_desk(&mut self) -> Task<Message> {
        let animated = self.config.appearance.animations.enabled;
        let enabled = self.config.desk.enabled;
        let unfolding = self.unfolding_response();

        let screens: Vec<(iced::SurfaceId, Option<String>)> = self
            .outputs
            .desk_surfaces()
            .map(|(id, screen)| (id, screen.map(ToOwned::to_owned)))
            .collect();

        let mut tasks = Vec::with_capacity(screens.len());

        for (surface, screen) in screens {
            let unfolded = enabled && self.desk.covers(screen.as_deref());
            let clock = self.desk_clocks.entry(screen).or_default();
            let was_out = clock.is_out();

            if unfolded {
                if !was_out {
                    if animated {
                        *clock = hydebar_core::animation::Unfold::default();
                        clock.advance(std::time::Duration::from_millis(1), unfolding);
                    } else {
                        clock.open();
                    }
                }
            } else {
                clock.fold();
            }

            if unfolded != was_out {
                if unfolded {
                    self.flip.borrow_mut().depart();
                } else {
                    self.send_the_islands_home(surface);
                }

                tasks.push(set_input_region(
                    surface,
                    if unfolded { None } else { Some(Vec::new()) }
                ));
            }
        }

        Task::batch(tasks)
    }

    /// How long the whole unfolding takes.
    ///
    /// The time one block is given to cross the screen and open, which is the
    /// time the whole bar takes: every block leaves at the same instant, so
    /// what a full bar costs over an empty one is nothing at all. The theme
    /// names it, and it is held between a snap and an unfolding that outstays
    /// its welcome.
    fn unfolding_response(&self) -> Duration {
        Duration::from_millis(self.config.appearance.animations.desk_block_ms)
            .clamp(SHORTEST_UNFOLDING, LONGEST_UNFOLDING)
    }

    /// Sends every island of `surface` off the screen and lets it fly back.
    ///
    /// The way home: the canvas is gone the moment a window takes the screen,
    /// and the strip it left behind would otherwise be simply there. Instead
    /// each island is seated beyond the edge it belongs to — the left hand
    /// section past the left edge, the right hand one past the right — and
    /// the strip's own travel carries them in to their places.
    ///
    /// Seated module by module, not island by island: on the strip every
    /// module holds a seat of its own, and the pill around a group is painted
    /// around wherever its modules are. Seating the group under one key left
    /// every module but the first without a seat to fly from, and they simply
    /// appeared while their neighbour flew.
    pub(crate) fn send_the_islands_home(&mut self, surface: iced::SurfaceId) {
        if !self.config.appearance.animations.enabled {
            return;
        }

        let beyond = self.screen_width.unwrap_or(1920.0);
        let layout = std::sync::Arc::clone(&self.config);

        {
            let mut memo = self.flip.borrow_mut();

            for (section, from) in [
                (&layout.modules.left, -beyond),
                (&layout.modules.center, -beyond),
                (&layout.modules.right, beyond * 2.0)
            ] {
                for module in Self::desk_order(section, false) {
                    memo.record(self.flip_key(module, surface), from);
                }
            }
        }

        self.flip.borrow_mut().depart();

        self.relayout = hydebar_core::animation::Spring::new(0.0)
            .with_response(hydebar_core::animation::STANDARD);
        self.relayout.set_target(1.0);
    }

    /// How far the canvas of `screen` has unfolded, zero folded and one out.
    #[must_use]
    pub(crate) fn desk_presence(&self, screen: Option<&str>) -> f32 {
        self.desk_clocks
            .get(&screen.map(ToOwned::to_owned))
            .map_or(0.0, |clock| clock.progress())
    }

    /// Advances every screen's unfolding by one frame.
    ///
    /// Reports whether any of them is still travelling.
    pub(crate) fn advance_desk(&mut self, elapsed: std::time::Duration) -> bool {
        let total = self.unfolding_response();
        let bare: Vec<Option<String>> = self
            .desk_clocks
            .keys()
            .filter(|screen| self.desk_covers(screen.as_deref()))
            .cloned()
            .collect();

        bare.into_iter().fold(false, |running, screen| {
            self.desk_clocks
                .get_mut(&screen)
                .is_some_and(|clock| clock.advance(elapsed, total))
                | running
        })
    }

    /// Reports whether the canvas belongs on `screen` at all.
    ///
    /// A clock only runs while its own screen is bare. Left to run on every
    /// screen it would carry a folded canvas out over a window: the clock of
    /// a screen holding one is at rest, not at zero speed, and a frame tick
    /// is not a reason to move it.
    #[must_use]
    fn desk_covers(&self, screen: Option<&str>) -> bool {
        self.config.desk.enabled && self.desk.covers(screen)
    }

    /// Reports whether any screen is in the middle of unfolding.
    #[must_use]
    pub(crate) fn desk_is_unfolding(&self) -> bool {
        self.desk_clocks.values().any(|clock| clock.is_running())
    }

    /// Reports whether the islands have left the strip of `screen`.
    ///
    /// The one question the strip and the canvas both ask, so that exactly
    /// one of them draws a module: the strip keeps what has not set off, and
    /// the canvas takes it the moment it does. Asking it twice, each in its
    /// own terms, is how a module came to be drawn on the canvas underneath a
    /// strip that was still standing over it.
    ///
    /// Asked of the whole bar rather than of one module, because the whole
    /// bar leaves at once: an island that waited its turn while its
    /// neighbours flew is the thing this unfolding does not do.
    #[must_use]
    pub(crate) fn has_left_the_strip(&self, screen: Option<&str>) -> bool {
        hydebar_core::animation::share(self.desk_presence(screen), 1.0).0 > 0.0
    }

    /// Reports whether the strip still has its islands standing on it.
    #[must_use]
    pub(crate) fn strip_still_holds(&self, screen: Option<&str>) -> bool {
        !self.has_left_the_strip(screen)
    }

    /// How much of the strip's own background is still painted on `screen`.
    ///
    /// The compositor blurs what shows through the strip, and it decides that
    /// from the pixels the strip paints: there is no half a blur to fade, only
    /// a surface worth blurring or none. Dropping the whole background on the
    /// frame the islands set off therefore switched the blur off like a
    /// light. The background goes out over the first stretch of the unfolding
    /// instead, so by the frame the compositor stops blurring there is nothing
    /// left on the strip to see the difference on.
    ///
    /// The way back is not this: a strip returning under a window that has
    /// already mapped takes its blur back at once, which is what it was asked
    /// for.
    #[must_use]
    pub(crate) fn strip_wash(&self, screen: Option<&str>) -> f32 {
        if !self.desk_holds(screen) {
            return 1.0;
        }

        1.0 - (self.desk_presence(screen) / WASH_GOES_OUT_BY).clamp(0.0, 1.0)
    }

    /// Reports whether the canvas, not the strip, holds `screen`.
    ///
    /// The one question both surfaces ask, and the reason they ask the same
    /// one: the strip and the canvas are two shapes of a single thing, so a
    /// frame showing both of them, or neither, is a frame that lies. Deciding
    /// it twice — once per surface, each against its own threshold on a
    /// travelling spring — is exactly how such a frame gets drawn.
    ///
    /// The canvas holds the screen for as long as its blocks are anywhere but
    /// home: from the first pixel of the travel out to the last of the travel
    /// back. The strip draws whenever it does not.
    #[must_use]
    pub(crate) fn desk_holds(&self, screen: Option<&str>) -> bool {
        self.config.desk.enabled && self.desk_presence(screen) > 0.0
    }
}
