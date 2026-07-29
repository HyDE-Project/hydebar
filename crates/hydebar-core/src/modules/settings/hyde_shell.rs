//! Commands the settings window drives the HyDE desktop with.
//!
//! HyDE exposes every desktop action through the `hyde-shell` dispatcher, so
//! the bar asks for a change the same way the user's own keybindings do instead
//! of writing HyDE's state files behind its back. That keeps the whole switch —
//! wallpaper, colours, the other clients — in HyDE's hands, and it means the
//! bar stays correct when HyDE changes how a switch is performed.
//!
//! The commands are built here, as plain strings, so the shape of an invocation
//! can be checked without a HyDE install present.

/// Command switching the desktop to `theme`.
///
/// Theme names carry spaces and accents (`Rosé Pine`), so the name is quoted
/// rather than interpolated: unquoted, `hyde-shell` would see a theme named
/// `Rosé` followed by a stray argument and fall back to the current theme.
#[must_use]
pub(super) fn switch_theme(theme: &str) -> String {
    format!("hyde-shell theme.switch -s {}", quote(theme))
}

/// Command moving the desktop to the next wallpaper of the active theme.
#[must_use]
pub(super) fn next_wallpaper() -> String {
    "hyde-shell wallpaper --next".to_owned()
}

/// Wraps `value` so a shell passes it on as a single, literal argument.
///
/// Single quotes are used because they suppress every expansion; the only
/// character they cannot carry is the single quote itself, which is spliced
/// back in the way a shell requires.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_theme_name_is_passed_as_one_quoted_argument() {
        assert_eq!(
            switch_theme("Gruvbox Retro"),
            "hyde-shell theme.switch -s 'Gruvbox Retro'"
        );
    }

    #[test]
    fn an_accented_theme_name_survives_quoting() {
        assert_eq!(
            switch_theme("Rosé Pine"),
            "hyde-shell theme.switch -s 'Rosé Pine'"
        );
    }

    #[test]
    fn a_quote_inside_a_theme_name_cannot_escape_the_argument() {
        assert_eq!(quote("it's; rm -rf /"), r"'it'\''s; rm -rf /'");
    }

    #[test]
    fn the_wallpaper_command_asks_for_the_next_one() {
        assert_eq!(next_wallpaper(), "hyde-shell wallpaper --next");
    }
}
