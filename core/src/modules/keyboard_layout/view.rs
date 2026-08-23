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
