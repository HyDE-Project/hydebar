//! The unfolding of the desk, screen by screen.

use hydebar_core::animation::{GENTLE, STANDARD};
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

        let screens: Vec<(iced::SurfaceId, Option<String>)> = self
            .outputs
            .desk_surfaces()
            .map(|(id, screen)| (id, screen.map(ToOwned::to_owned)))
            .collect();

        let mut tasks = Vec::with_capacity(screens.len());

        for (surface, screen) in screens {
            let unfolded = enabled && self.desk.covers(screen.as_deref());
            let was_out = self.desk_fades.progress(&screen) > 0.0;

            self.desk_fades
                .point(screen, unfolded, animated && unfolded, GENTLE);

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

    /// Advances the opening of every block that has finished travelling.
    ///
    /// The two halves of the unfolding are stated apart on purpose. A module
    /// crosses the screen in the shape the strip gave it — a glance, a pill —
    /// and only once it has come to rest does it write out everything it
    /// knows. Starting both at once would have the blocks growing while they
    /// are still moving, which reads as the layout thrashing rather than as
    /// the bar unfolding.
    ///
    /// Reports whether any opening still needs frames.
    pub(crate) fn advance_desk_blooms(&mut self, elapsed: std::time::Duration) -> bool {
        let animated = self.config.appearance.animations.enabled;

        let screens: Vec<Option<String>> = self
            .outputs
            .desk_surfaces()
            .map(|(_, screen)| screen.map(ToOwned::to_owned))
            .collect();

        for screen in screens {
            let arrived =
                self.desk_holds(screen.as_deref()) && self.desk_has_landed(screen.as_ref());

            self.desk_blooms
                .point(screen, arrived, animated && arrived, STANDARD);
        }

        self.desk_blooms.advance(elapsed)
    }

    /// Sends every island of `surface` off the screen and lets it fly back.
    ///
    /// The way home: the canvas is gone the moment a window takes the screen,
    /// and the strip it left behind would otherwise be simply there. Instead
    /// each island is seated beyond the edge it belongs to — the left hand
    /// section past the left edge, the right hand one past the right — and
    /// the strip's own travel carries them in to their places.
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

    /// Reports whether the blocks of `screen` have come to rest.
    fn desk_has_landed(&self, screen: Option<&String>) -> bool {
        self.desk_fades.progress(&screen.cloned()) >= 1.0
    }

    /// Reports whether any screen has blocks waiting to be written out.
    ///
    /// The frame clock has to keep running through the pause between the two
    /// halves: the travel has settled, so its spring asks for no more frames,
    /// and the opening has not started, so neither does it.
    #[must_use]
    pub(crate) fn desk_blooms_are_due(&self) -> bool {
        self.outputs.desk_surfaces().any(|(_, screen)| {
            let screen = screen.map(ToOwned::to_owned);

            self.desk_holds(screen.as_deref())
                && self.desk_has_landed(screen.as_ref())
                && self.desk_blooms.progress(&screen) < 1.0
        })
    }

    /// How far the blocks of `screen` have written themselves out.
    #[must_use]
    pub(crate) fn desk_bloom(&self, screen: Option<&str>) -> f32 {
        self.desk_blooms.progress(&screen.map(ToOwned::to_owned))
    }

    /// How far the canvas of `screen` has unfolded, zero folded and one out.
    #[must_use]
    pub(crate) fn desk_presence(&self, screen: Option<&str>) -> f32 {
        self.desk_fades.progress(&screen.map(ToOwned::to_owned))
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
