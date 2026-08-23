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
