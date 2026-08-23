//! Drawing of the window title entry.

use iced::{Element, widget::text};

use super::{WindowTitle, state::shown_title};
use crate::{components::scale, config::WindowTitleConfig, modules::OnModulePress};

impl WindowTitle {
    /// The bar entry: the focused window's title, in full while the module is
    /// attended.
    ///
    /// A title long enough to be shortened is exactly the one the user leans
    /// in to read, so looking at the module is taken as asking for the rest of
    /// it.
    ///
    /// Rendered by the module itself, so the bar layer holds no title drawing
    /// of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        config: &WindowTitleConfig,
        attended: bool
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static
    {
        let value = self.value.as_ref()?;

        let shown = if attended {
            value.clone()
        } else {
            self.shortened
                .clone()
                .unwrap_or_else(|| shown_title(value, config, attended))
        };

        Some((
            text(shown)
                .size(scale::scaled(12.0))
                .wrapping(text::Wrapping::WordOrGlyph)
                .into(),
            None
        ))
    }
}
