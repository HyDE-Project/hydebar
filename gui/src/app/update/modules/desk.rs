//! The unfolding of the desk, screen by screen.

use std::time::Duration;

use hydebar_core::config::ModuleName;

/// The briefest the whole unfolding is allowed to be.
const SHORTEST_UNFOLDING: Duration = Duration::from_millis(600);

/// The longest the whole unfolding is allowed to be.
const LONGEST_UNFOLDING: Duration = Duration::from_millis(2400);
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

    /// How long the whole unfolding takes, given how much there is to unfold.
    ///
    /// The blocks travel one at a time, so a layout of a dozen modules has a
    /// dozen flights to fit in: a fixed duration would either rush a full bar
    /// into a blur or leave a bar of three modules crawling. Each block is
    /// given the share the theme names, and the whole is held between a quick
    /// unfolding and one that outstays its welcome.
    fn unfolding_response(&self) -> Duration {
        let layout = &self.config.modules;
        let blocks = layout.left.len() + layout.center.len() + layout.right.len();

        Duration::from_millis(self.config.appearance.animations.desk_block_ms)
            .saturating_mul(u32::try_from(blocks).unwrap_or(u32::MAX))
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

    /// Reports whether `module` has already left the strip of `screen`.
    ///
    /// The one question the strip and the canvas both ask about a module, so
    /// that exactly one of them draws it: the strip keeps what has not set
    /// off yet, and the canvas takes it the moment it does. Asking it twice,
    /// each in its own terms, is how a module came to be drawn on the canvas
    /// underneath a strip that was still standing over it.
    #[must_use]
    pub(crate) fn has_left_the_strip(&self, module: &ModuleName, screen: Option<&str>) -> bool {
        let Some((turn, units)) = self.strip_turn(module) else {
            return true;
        };

        #[expect(
            clippy::cast_precision_loss,
            reason = "a layout holds a handful of units"
        )]
        let place = if units > 1 {
            turn as f32 / (units - 1) as f32
        } else {
            0.0
        };

        hydebar_core::animation::share(self.desk_presence(screen), place, units).0 > 0.0
    }

    /// The turn `module` takes in the unfolding, and how many turns there are.
    fn strip_turn(&self, module: &ModuleName) -> Option<(usize, usize)> {
        let (left, centre, right, units) = Self::desk_turns(&self.config.modules);

        left.into_iter()
            .chain(centre)
            .chain(right)
            .find(|(_, standing)| *standing == module)
            .map(|(turn, _)| (turn, units))
    }

    /// Reports whether the strip still has a module standing on it.
    ///
    /// The strip does not go the moment the first island leaves it: the
    /// islands leave one at a time, and until the last one has set off there
    /// is still something standing on the strip. The strip stays under them,
    /// empty of everything that has already gone, and leaves once the last
    /// has.
    #[must_use]
    pub(crate) fn strip_still_holds(&self, screen: Option<&str>) -> bool {
        let layout = &self.config.modules;

        [&layout.left, &layout.center, &layout.right]
            .into_iter()
            .flat_map(|section| Self::desk_order(section, false))
            .any(|module| !self.has_left_the_strip(module, screen))
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
