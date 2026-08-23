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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::Arc;

    use hydebar_proto::ports::hyprland::{HyprlandPort, HyprlandWindowInfo};

    use super::*;
    use crate::{modules::window_title::Message, test_utils::MockHyprlandPort};

    fn module() -> WindowTitle {
        let port: Arc<dyn HyprlandPort> = Arc::new(MockHyprlandPort::default());

        WindowTitle::new(port, &WindowTitleConfig::default())
    }

    fn window(title: &str) -> HyprlandWindowInfo {
        HyprlandWindowInfo {
            title: title.to_owned(),
            class: "term".to_owned()
        }
    }

    #[test]
    fn a_bare_workspace_carries_no_title() {
        let mut title = module();
        title.update(Message::TitleChanged(None), &WindowTitleConfig::default());

        assert!(
            title
                .bar_view::<()>(&WindowTitleConfig::default(), false)
                .is_none()
        );
    }

    #[test]
    fn a_focused_window_puts_its_title_on_the_strip() {
        let mut title = module();
        title.update(
            Message::TitleChanged(Some(window("a window"))),
            &WindowTitleConfig::default()
        );

        assert!(
            title
                .bar_view::<()>(&WindowTitleConfig::default(), false)
                .is_some()
        );
    }
}
