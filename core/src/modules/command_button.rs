//! One icon that runs a configured command.
//!
//! The launcher and clipboard entries are the same shape: an icon drawn while
//! its command is configured, pressed to run that command. Each bar entry owns
//! its instance, so the entries stay separate on the bar while sharing one
//! implementation. The press action is named by the GUI layer, which knows the
//! message to send.

use iced::Element;

use super::{Module, OnModulePress};
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

impl<M> Module<M> for CommandButton
where
    M: 'static + Clone
{
    type ViewData<'a> = (&'a Option<String>, &'a IconTheme);
    type RegistrationData<'a> = ();

    fn view(
        &self,
        (command, icons): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
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

        let result =
            <CommandButton as Module<()>>::view(&button, (&command, &IconTheme::default()));
        assert!(result.is_some());

        if let Some((_, action)) = result {
            assert!(action.is_none());
        }
    }

    #[test]
    fn view_returns_none_when_command_absent() {
        let button = CommandButton::new(Icons::Clipboard);
        let command = None;

        let result =
            <CommandButton as Module<()>>::view(&button, (&command, &IconTheme::default()));
        assert!(result.is_none());
    }
}
