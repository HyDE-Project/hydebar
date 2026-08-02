//! One-press buttons for the desktop's own helpers.
//!
//! Each entry is an icon whose press hands one fixed command to the
//! `hyde-shell` dispatcher: the keybinding cheatsheet, the night-light
//! toggle, the game-mode toggle. The desktop owns the behaviour and the
//! state; the bar only offers the switch, so the entries hold nothing and
//! follow the render-function convention.

use iced::Element;

use super::OnModulePress;
use crate::{
    components::icons::{IconTheme, Icons, icon},
    utils::hyde_shell
};

/// The desktop helpers the bar offers a one-press switch for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HydeButton {
    /// Shows the keybinding cheatsheet.
    KeybindHint,
    /// Toggles the night light.
    NightLight,
    /// Toggles game mode.
    GameMode
}

impl HydeButton {
    /// The glyph the entry is drawn with.
    #[must_use]
    pub const fn glyph(self) -> Icons {
        match self {
            Self::KeybindHint => Icons::KeybindHint,
            Self::NightLight => Icons::NightLight,
            Self::GameMode => Icons::GameMode
        }
    }

    /// The command the press hands to the dispatcher.
    #[must_use]
    pub fn command(self) -> String {
        match self {
            Self::KeybindHint => hyde_shell::keybinds_hint(),
            Self::NightLight => hyde_shell::toggle_night_light(),
            Self::GameMode => hyde_shell::toggle_game_mode()
        }
    }
}

/// The bar entry: the helper's glyph, pressed to run its command.
///
/// `run` names the message carrying a command to the shell; the GUI layer
/// owns that vocabulary.
#[must_use]
pub fn bar_view<M>(
    button: HydeButton,
    icons: &IconTheme,
    run: impl Fn(String) -> M
) -> (Element<'static, M>, Option<OnModulePress<M>>)
where
    M: 'static + Clone
{
    (
        icon(icons, button.glyph()).into(),
        Some(OnModulePress::Action(Box::new(run(button.command()))))
    )
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn every_button_speaks_through_the_dispatcher() {
        for button in [
            HydeButton::KeybindHint,
            HydeButton::NightLight,
            HydeButton::GameMode
        ] {
            assert!(button.command().starts_with("hyde-shell "));
        }
    }

    #[test]
    fn every_button_owns_a_distinct_glyph() {
        assert_ne!(
            HydeButton::KeybindHint.glyph(),
            HydeButton::NightLight.glyph()
        );
        assert_ne!(HydeButton::NightLight.glyph(), HydeButton::GameMode.glyph());
        assert_ne!(
            HydeButton::KeybindHint.glyph(),
            HydeButton::GameMode.glyph()
        );
    }

    #[test]
    fn a_press_carries_the_button_command() {
        let (_, action) =
            bar_view::<String>(HydeButton::GameMode, &IconTheme::default(), |command| {
                command
            });

        match action {
            Some(OnModulePress::Action(command)) => {
                assert_eq!(*command, hyde_shell::toggle_game_mode());
            }
            _ => panic!("expected the game mode command action")
        }
    }
}
