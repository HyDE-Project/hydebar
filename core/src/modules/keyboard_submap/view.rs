//! Drawing of the submap entry: the name of the mode in force.

use iced::{Element, widget::text};

use super::KeyboardSubmap;
use crate::{components::scale, modules::OnModulePress};

impl KeyboardSubmap {
    /// The bar entry: the name of the submap the keyboard is in.
    ///
    /// Draws nothing while the compositor is in no submap, which is most of
    /// the session.
    ///
    /// Rendered by the module itself, so the bar layer holds no submap
    /// drawing of its own.
    #[must_use]
    pub fn bar_view<M>(&self) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + Clone
    {
        if self.submap.is_empty() {
            return None;
        }

        let label = if self.shown.current().is_empty() {
            text(self.submap.clone()).into()
        } else {
            self.shown.element(scale::base())
        };

        Some((label, None))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::Arc;

    use hydebar_proto::ports::hyprland::{HyprlandKeyboardState, HyprlandPort};

    use super::*;
    use crate::{modules::keyboard_submap::Message, test_utils::MockHyprlandPort};

    fn module(submap: Option<&str>) -> KeyboardSubmap {
        let port = MockHyprlandPort::default();
        *port.keyboard_state.lock().expect("keyboard lock") = HyprlandKeyboardState {
            active_layout:        "us".into(),
            has_multiple_layouts: false,
            active_submap:        submap.map(ToOwned::to_owned)
        };

        KeyboardSubmap::new(Arc::new(port) as Arc<dyn HyprlandPort>)
    }

    #[test]
    fn a_keyboard_in_no_submap_draws_nothing() {
        assert!(module(None).bar_view::<()>().is_none());
    }

    #[test]
    fn a_keyboard_in_a_submap_names_it() {
        assert!(module(Some("resize")).bar_view::<()>().is_some());
    }

    #[test]
    fn leaving_a_submap_takes_the_entry_off_the_strip() {
        let mut submap = module(Some("resize"));
        submap.update(Message::SubmapChanged(String::new()), false);

        assert!(submap.bar_view::<()>().is_none());
    }
}
