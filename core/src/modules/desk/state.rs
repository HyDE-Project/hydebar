//! Message folding for the desk.

use super::{Desk, Message};

impl Desk {
    /// Folds a fresh answer about the screens in.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::ScreensChanged(bareness) => self.bareness = bareness
        }
    }

    /// Reports whether the desk unfolds on the surface of `monitor`.
    ///
    /// A surface with no monitor name is the fallback one the bar runs on
    /// until the compositor reports its screens; it answers for the focused
    /// screen.
    #[must_use]
    pub fn covers(&self, monitor: Option<&str>) -> bool {
        self.bareness.covers(monitor)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::Arc;

    use hydebar_proto::ports::hyprland::HyprlandPort;

    use super::{Desk, Message};
    use crate::{modules::desk::test_bareness, test_utils::MockHyprlandPort};

    #[test]
    fn a_fresh_answer_replaces_the_one_on_screen() {
        let port = Arc::new(MockHyprlandPort::default()) as Arc<dyn HyprlandPort>;
        let mut desk = Desk::new(port);

        assert!(!desk.covers(Some("DP-1")));

        desk.update(Message::ScreensChanged(test_bareness()));

        assert!(desk.covers(Some("DP-1")));
        assert!(!desk.covers(Some("HDMI-A-1")));
    }
}
