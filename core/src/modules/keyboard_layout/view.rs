//! Drawing of the keyboard layout entry: the label of the active layout.

use iced::{Element, widget::text};

use super::KeyboardLayout;
use crate::{components::scale, config::KeyboardLayoutModuleConfig, modules::OnModulePress};

impl KeyboardLayout {
    /// The bar entry: the label the configuration gives the active layout.
    ///
    /// Draws nothing at all when the compositor has a single layout — an
    /// indicator of a choice nobody can make is noise on the strip.
    ///
    /// Rendered by the module itself, so the bar layer holds no layout
    /// drawing of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        config: &KeyboardLayoutModuleConfig
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + Clone
    {
        if !self.multiple_layout {
            return None;
        }

        let label = if self.shown.current().is_empty() {
            let active = config
                .labels
                .get(&self.active)
                .map_or_else(|| self.active.clone(), Clone::clone);

            text(active).into()
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
    use crate::test_utils::MockHyprlandPort;

    fn module(multiple: bool) -> KeyboardLayout {
        let port = MockHyprlandPort::default();
        *port.keyboard_state.lock().expect("keyboard lock") = HyprlandKeyboardState {
            active_layout:        "us".into(),
            has_multiple_layouts: multiple,
            active_submap:        None
        };

        KeyboardLayout::new(Arc::new(port) as Arc<dyn HyprlandPort>)
    }

    #[test]
    fn a_single_layout_is_no_choice_and_draws_nothing() {
        assert!(
            module(false)
                .bar_view::<()>(&KeyboardLayoutModuleConfig::default())
                .is_none()
        );
    }

    #[test]
    fn several_layouts_put_the_active_one_on_the_strip() {
        assert!(
            module(true)
                .bar_view::<()>(&KeyboardLayoutModuleConfig::default())
                .is_some()
        );
    }
}
