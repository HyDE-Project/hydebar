//! Parser for the variables a Hyprland fragment declares.
//!
//! A `HyDE` theme states the choices that are not colours — the bar font, the
//! GTK theme, the light/dark preference — as hyprlang variables inside
//! `hypr.theme`: `$BAR_FONT = JetBrainsMono Nerd Font`. `HyDE` itself reads
//! them with `hyq`, a whole config parser, but the bar only ever wants the
//! right hand side of a handful of names, so a line scan is enough and costs no
//! process.
//!
//! The grammar differs from [`crate::shell_vars`] in two ways that matter: the
//! name carries a leading `$`, and the value is unquoted and may contain
//! spaces, so it runs to the end of the line instead of stopping at the first
//! word.

/// Value assigned to a hyprlang variable, without its `$`.
///
/// `name` is given without the sigil (`BAR_FONT`) because that is how the value
/// is referred to everywhere else; the sigil is a detail of the file.
///
/// The last assignment wins, mirroring hyprlang: a fragment that re-declares a
/// variable means the later line.
#[must_use]
pub fn value_of(source: &str, name: &str) -> Option<String> {
    let mut found = None;

    for line in source.lines() {
        let Some((declared, value)) = assignment(line) else {
            continue;
        };

        if declared == name {
            found = Some(value.to_owned());
        }
    }

    found.filter(|value| !value.is_empty())
}

/// Splits a line into the variable it declares and the value, if it declares
/// one.
fn assignment(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    let name = line.strip_prefix('$')?;
    let (name, value) = name.split_once('=')?;
    let name = name.trim();

    if name.is_empty() || !name.chars().all(is_name_char) {
        return None;
    }

    Some((name, strip_comment(value)))
}

/// Whether `c` may appear in a hyprlang variable name.
const fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Drops a trailing `#` comment and the whitespace around the value.
///
/// A value keeps its inner spaces: font families are written bare, so
/// `JetBrainsMono Nerd Font` has to survive intact.
fn strip_comment(value: &str) -> &str {
    value
        .find('#')
        .map_or_else(|| value.trim(), |start| value[..start].trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HYPR_THEME: &str = "\
$GTK_THEME = Gruvbox-Retro
$COLOR_SCHEME = prefer-dark
$BAR_FONT = JetBrainsMono Nerd Font
$BAR_FONT_SIZE = 11
# $BAR_FONT = Commented Out

general {
    gaps_in = 3
}
";

    #[test]
    fn a_declared_variable_is_read_without_its_sigil() {
        assert_eq!(
            value_of(HYPR_THEME, "GTK_THEME"),
            Some("Gruvbox-Retro".to_owned())
        );
    }

    #[test]
    fn a_value_keeps_the_spaces_inside_it() {
        assert_eq!(
            value_of(HYPR_THEME, "BAR_FONT"),
            Some("JetBrainsMono Nerd Font".to_owned())
        );
    }

    #[test]
    fn a_commented_out_declaration_is_ignored() {
        assert_eq!(value_of("# $BAR_FONT = Nope", "BAR_FONT"), None);
    }

    #[test]
    fn a_plain_keyword_is_not_read_as_a_variable() {
        assert_eq!(value_of(HYPR_THEME, "gaps_in"), None);
        assert_eq!(assignment("    gaps_in = 3"), None);
    }

    #[test]
    fn a_trailing_comment_stays_out_of_the_value() {
        assert_eq!(
            value_of("$BAR_FONT = Iosevka # the default", "BAR_FONT"),
            Some("Iosevka".to_owned())
        );
    }

    #[test]
    fn a_variable_that_was_never_declared_reads_as_missing() {
        assert_eq!(value_of(HYPR_THEME, "MENU_FONT"), None);
    }

    #[test]
    fn an_empty_declaration_reads_as_missing() {
        assert_eq!(value_of("$BAR_FONT =\n", "BAR_FONT"), None);
    }

    #[test]
    fn the_last_declaration_of_a_variable_wins() {
        assert_eq!(
            value_of("$BAR_FONT = Old\n$BAR_FONT = New\n", "BAR_FONT"),
            Some("New".to_owned())
        );
    }
}
