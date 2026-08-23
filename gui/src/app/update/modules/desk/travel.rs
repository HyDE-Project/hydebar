//! What the strip and the canvas each ask of the other while one leaves.
//!
//! The two shapes of the bar answer the same questions from opposite sides,
//! and every one of them is settled here so a frame showing both of them, or
//! neither, cannot be drawn.

use hydebar_core::animation::Journey;

use super::super::super::super::state::App;

impl App {
    /// Reports whether the canvas belongs on `screen` at all.
    ///
    /// A clock only runs while its own screen is bare. Left to run on every
    /// screen it would carry a folded canvas out over a window: the clock of
    /// a screen holding one is at rest, not at zero speed, and a frame tick
    /// is not a reason to move it.
    #[must_use]
    pub(super) fn desk_covers(&self, screen: Option<&str>) -> bool {
        self.config.desk.enabled && self.desk.covers(screen)
    }

    /// Reports whether any screen is in the middle of unfolding.
    #[must_use]
    pub(crate) fn desk_is_unfolding(&self) -> bool {
        self.desk_returning.is_running()
            || self.desk_leaving.is_running()
            || self.desk_clocks.values().any(|clock| clock.is_running())
    }

    /// Reports whether the block of `journey` has left the strip of `screen`.
    ///
    /// The one question the strip and the canvas both ask, so that exactly
    /// one of them draws a module: the strip keeps what has not set off, and
    /// the canvas takes it the moment it does. Asking it twice, each in its
    /// own terms, is how a module came to be drawn on the canvas underneath a
    /// strip that was still standing over it.
    ///
    /// Asked of one block rather than of the whole bar, because the bar
    /// leaves in a run: the block nearest its place goes first and the
    /// furthest last, and each is handed over on the frame its own turn
    /// comes.
    #[must_use]
    pub(crate) fn has_left_the_strip(&self, screen: Option<&str>, journey: Journey) -> bool {
        hydebar_core::animation::share(self.desk_presence(screen), journey).0 > 0.0
    }

    /// Reports whether the strip still has an island of its own standing.
    ///
    /// Asked of the last block to go: while it is still there the strip has
    /// something to draw, and a strip that stopped drawing at the first
    /// departure would take the rest of the run with it.
    #[must_use]
    pub(crate) fn strip_still_holds(&self, screen: Option<&str>) -> bool {
        !self.has_left_the_strip(screen, Journey::whole())
    }

    /// How much of the strip's own background is still painted on `screen`.
    ///
    /// The compositor blurs what shows through the strip, and it decides that
    /// from the pixels the strip paints: there is no half a blur to fade, only
    /// a surface worth blurring or none. Dropping the whole background on the
    /// frame the islands set off therefore switched the blur off like a
    /// light.
    ///
    /// It goes out behind the islands instead, never ahead of them: the
    /// background is still standing where a module has yet to land, and the
    /// place it held is bare from the moment it does. The near islands land
    /// first and the ones from the ends of the bar last, so what the strip
    /// does is open from the middle outwards at the pace its own islands come
    /// down — and by the frame the compositor stops blurring, the last of it
    /// has gone from under the last island.
    ///
    /// The way back runs the same picture backwards: the two ends of the
    /// strip are painted first and the opening closes towards the middle, so
    /// the blur is back before the islands that fly in over it. It leads them
    /// rather than arriving with them — a background still closing under a
    /// module already in its place is the one order this must not have.
    #[must_use]
    pub(crate) fn strip_wash(&self, screen: Option<&str>) -> f32 {
        if !self.desk_holds(screen) || self.desk_leaving(screen).is_some() {
            return self.desk_returning.progress();
        }

        let nearest = hydebar_core::animation::landed(Self::journey(0, self.deepest_column()));
        let furthest = hydebar_core::animation::landed(Journey::whole());
        let span = (furthest - nearest).max(f32::EPSILON);

        1.0 - ((self.desk_presence(screen) - nearest) / span).clamp(0.0, 1.0)
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
