//! One icon that runs a configured command.
//!
//! The launcher and clipboard entries are the same shape: an icon drawn while
//! its command is configured, pressed to run that command. Each bar entry owns
//! its instance, so the entries stay separate on the bar while sharing one
//! implementation. The press action is named by the GUI layer, which knows the
//! message to send.

use iced::Element;

use super::OnModulePress;
use crate::components::icons::{IconTheme, Icons, icon};

/// A stateless bar entry showing `glyph` while a command is configured.
#[derive(Debug, Clone)]
pub struct CommandButton {
    glyph: Icons
}

impl CommandButton {
    /// Creates an entry drawn with `glyph`.
    #[must_use]
    pub const fn new(glyph: Icons) -> Self {
        Self {
            glyph
        }
    }
}

impl CommandButton {
    /// The bar entry: the glyph, drawn only while a command is configured.
    ///
    /// The press is named by the bar layer, which knows the message the entry
    /// sends; an entry with no command to run is not drawn at all.
    ///
    /// Rendered by the module itself, so the bar layer holds no button
    /// drawing of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        command: &Option<String>,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static
    {
        command
            .as_ref()
            .map(|_| (icon(icons, self.glyph).into(), None))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::components::icons::IconTheme;

    #[test]
    fn view_returns_some_when_command_present() {
        let button = CommandButton::new(Icons::AppLauncher);
        let command = Some("wofi".to_string());

        let result = button.bar_view::<()>(&command, &IconTheme::default());
        assert!(result.is_some());

        if let Some((_, action)) = result {
            assert!(action.is_none());
        }
    }

    #[test]
    fn view_returns_none_when_command_absent() {
        let button = CommandButton::new(Icons::Clipboard);
        let command = None;

        let result = button.bar_view::<()>(&command, &IconTheme::default());
        assert!(result.is_none());
    }
}
