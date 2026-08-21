//! The unfolding of the desk, screen by screen.

use hydebar_core::animation::GENTLE;

use super::super::super::state::App;

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
    pub(crate) fn unfold_desk(&mut self) {
        let animated = self.config.appearance.animations.enabled;
        let enabled = self.config.desk.enabled;

        let screens: Vec<Option<String>> = self
            .outputs
            .screens()
            .map(|screen| screen.map(ToOwned::to_owned))
            .collect();

        for screen in screens {
            let unfolded = enabled && self.desk.covers(screen.as_deref());

            self.desk_fades.point(screen, unfolded, animated, GENTLE);
        }
    }

    /// How far the canvas of `screen` has unfolded, zero folded and one out.
    #[must_use]
    pub(crate) fn desk_presence(&self, screen: Option<&str>) -> f32 {
        self.desk_fades.progress(&screen.map(ToOwned::to_owned))
    }
}
