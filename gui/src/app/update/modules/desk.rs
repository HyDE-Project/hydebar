//! The unfolding of the desk, screen by screen.

use std::time::Duration;

/// The briefest the whole unfolding is allowed to be.
const SHORTEST_UNFOLDING: Duration = Duration::from_millis(600);

/// The longest the whole unfolding is allowed to be.
const LONGEST_UNFOLDING: Duration = Duration::from_millis(2400);

/// How long the strip's background takes to close on the way back.
///
/// Shorter than the islands' own flight home, which settles a quarter of a
/// second after it starts: the background is what they land on, so it is
/// whole before they are, and long enough to be seen closing rather than
/// simply appearing.
const RETURN_WASH: Duration = Duration::from_millis(190);

/// How long the canvas takes to leave the screen sideways.
///
/// Shorter than the way out: the canvas leaves because a window has taken the
/// screen, and a canvas still sliding off a window the user is already
/// working in reads as the bar holding on. Long enough to be a leaving rather
/// than a disappearance.
const LEAVING: Duration = Duration::from_millis(260);

mod travel;

use iced::{Task, set_input_region};

use super::super::super::state::{App, Leaving, Message};

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
            let screen_of = screen.clone();
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
                    self.desk_leaving_from = Leaving::Nothing;
                    self.flip.borrow_mut().depart();
                } else {
                    self.send_the_islands_home(surface);
                    self.send_the_canvas_away(screen_of);
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
                    memo.record(
                        self.flip_key(module, surface),
                        iced::Rectangle::new(
                            iced::Point::new(from, 0.0),
                            iced::Size::new(0.0, 0.0)
                        )
                    );
                }
            }
        }

        self.flip.borrow_mut().depart();

        self.desk_returning = hydebar_core::animation::Unfold::default();
        self.desk_returning
            .advance(Duration::from_millis(1), RETURN_WASH);
        self.relayout = hydebar_core::animation::Spring::new(0.0)
            .with_response(hydebar_core::animation::STANDARD);
        self.relayout.set_target(1.0);
    }

    /// Starts the canvas of `screen` on its way off the two edges.
    ///
    /// The seats it travels to are the ones the strip's islands are flying in
    /// from — the same book, read the other way — so a block leaves by the
    /// edge its own section belongs to and arrives nowhere: the clock behind
    /// it is what takes it off the screen for good.
    fn send_the_canvas_away(&mut self, screen: Option<String>) {
        if !self.config.appearance.animations.enabled {
            self.desk_leaving_from = Leaving::Nothing;

            return;
        }

        self.desk_leaving = hydebar_core::animation::Unfold::default();
        self.desk_leaving.advance(Duration::from_millis(1), LEAVING);
        self.desk_leaving_from = Leaving::Screen(screen);
    }

    /// How far the canvas of `screen` has got on its way out, if it is on one.
    #[must_use]
    pub(crate) fn desk_leaving(&self, screen: Option<&str>) -> Option<f32> {
        let Leaving::Screen(leaving) = &self.desk_leaving_from else {
            return None;
        };

        if leaving.as_deref() != screen || !self.desk_leaving.is_running() {
            return None;
        }

        Some(self.desk_leaving.progress())
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
        let returning = self.desk_returning.advance(elapsed, RETURN_WASH);
        let leaving = self.desk_leaving.advance(elapsed, LEAVING);

        if !leaving {
            self.desk_leaving_from = Leaving::Nothing;
        }

        let total = self.unfolding_response();
        let bare: Vec<Option<String>> = self
            .desk_clocks
            .keys()
            .filter(|screen| self.desk_covers(screen.as_deref()))
            .cloned()
            .collect();

        bare.into_iter()
            .fold(returning | leaving, |running, screen| {
                self.desk_clocks
                    .get_mut(&screen)
                    .is_some_and(|clock| clock.advance(elapsed, total))
                    | running
            })
    }
}
