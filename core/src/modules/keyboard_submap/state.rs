//! Message folding for the submap indicator.

use std::time::Duration;

use super::{KeyboardSubmap, Message};

impl KeyboardSubmap {
    /// Applies what the compositor said.
    ///
    /// `animated` decides whether the shown submap dissolves into its
    /// replacement or swaps outright.
    pub fn update(&mut self, message: Message, animated: bool) {
        match message {
            Message::SubmapChanged(submap) => {
                self.submap = submap;
            }
        }

        self.shown.set(self.submap.clone(), animated);
    }

    /// Advances the dissolve of the shown submap.
    pub fn tick_fade(&mut self, elapsed: Duration) -> bool {
        self.shown.advance(elapsed)
    }

    /// Whether the shown submap is still dissolving.
    #[must_use]
    pub fn is_fading(&self) -> bool {
        self.shown.is_animating()
    }

    #[cfg(test)]
    pub(crate) fn submap(&self) -> &str {
        &self.submap
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::Arc;

    use hydebar_proto::ports::hyprland::HyprlandPort;

    use super::*;
    use crate::test_utils::MockHyprlandPort;

    fn module() -> KeyboardSubmap {
        let port: Arc<dyn HyprlandPort> = Arc::new(MockHyprlandPort::default());

        KeyboardSubmap::new(port)
    }

    #[test]
    fn initializes_with_port_submap() {
        assert_eq!(module().submap(), "resize");
    }

    #[test]
    fn update_replaces_submap_value() {
        let mut module = module();

        module.update(Message::SubmapChanged("launch".into()), false);

        assert_eq!(module.submap(), "launch");
    }
}
