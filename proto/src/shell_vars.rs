//! Parser for the shell assignments `HyDE` keeps most of its settings in.
//!
//! Every `HyDE` file the bar reads — `~/.local/state/hyde/staterc`,
//! `~/.config/hyde/config.toml`, `~/.local/share/hyde/env-theme` and the
//! `~/.cache/hyde/*.dcol` palettes — is a flat list of `KEY="value"` lines that
//! the scripts `source` rather than parse. One grammar therefore covers all of
//! them, which is why this lives beside the readers instead of inside any one
//! of them.
//!
//! Only that shape is understood: expansions, command substitutions and
//! anything else a shell would evaluate are left alone, because the bar reads
//! the files and never runs them.

/// Value assigned to `key`, with its quotes removed.
///
/// The last assignment wins, mirroring what sourcing the file in a shell would
/// leave behind: `HyDE` appends rather than rewrites when it records a change.
#[must_use]
pub fn value_of(source: &str, key: &str) -> Option<String> {
    let mut found = None;

    for line in source.lines() {
        let Some((name, value)) = assignment(line) else {
            continue;
        };

        if name == key {
            found = Some(unquote(value));
        }
    }

    found.filter(|value| !value.is_empty())
}

/// Value assigned to `key`, read as a number.
///
/// `HyDE` writes numbers as quoted strings (`enableWallDcol="1"`), so a caller
/// that needs the number would otherwise repeat the unquote-then-parse dance at
/// every call site and get the "written by hand as `1 `" case subtly wrong.
#[must_use]
pub fn number<T: std::str::FromStr>(source: &str, key: &str) -> Option<T> {
    value_of(source, key).and_then(|value| value.trim().parse().ok())
}

/// Splits a line into the name it assigns and the raw value, if it assigns one.
fn assignment(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();

    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
    let (name, value) = line.split_once('=')?;
    let name = name.trim();

    if name.is_empty() || !name.chars().all(is_name_char) {
        return None;
    }

    Some((name, value.trim()))
}

/// Whether `c` may appear in a shell variable name.
///
/// Names are checked rather than assumed so a line such as `foo bar = baz` or a
/// stray `case ... in` fragment is skipped instead of read as an assignment.
const fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Removes one layer of matching quotes from a raw value.
///
/// An unquoted value keeps only its first word: a shell would treat the rest as
/// further arguments, and a trailing comment would otherwise land in the value.
fn unquote(value: &str) -> String {
    let mut chars = value.chars();

    match (chars.next(), value.chars().last()) {
        (Some(open @ ('"' | '\'')), Some(close)) if open == close && value.len() >= 2 => {
            value[1..value.len() - 1].to_owned()
        }
        _ => value
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATERC: &str = r#"
# written by HyDE
HYDE_THEME="Gruvbox Retro"
HYPR_SHADER="wallbash"
enableWallDcol="1"
WAYBAR_LAYOUT_NAME=hyprdots/02
export HYDE_THEME_DIR="/home/user/.config/hyde/themes/Gruvbox Retro"
"#;

    #[test]
    fn a_quoted_value_is_read_without_its_quotes() {
        assert_eq!(
            value_of(STATERC, "HYDE_THEME"),
            Some("Gruvbox Retro".to_owned())
        );
    }

    #[test]
    fn an_unquoted_value_is_read_as_written() {
        assert_eq!(
            value_of(STATERC, "WAYBAR_LAYOUT_NAME"),
            Some("hyprdots/02".to_owned())
        );
    }

    #[test]
    fn an_exported_assignment_is_read_like_a_plain_one() {
        assert_eq!(
            value_of(STATERC, "HYDE_THEME_DIR"),
            Some("/home/user/.config/hyde/themes/Gruvbox Retro".to_owned())
        );
    }

    #[test]
    fn a_key_that_was_never_assigned_reads_as_missing() {
        assert_eq!(value_of(STATERC, "HYPR_NOPE"), None);
    }

    #[test]
    fn a_commented_out_assignment_is_ignored() {
        assert_eq!(value_of("# HYDE_THEME=\"Nordic Blue\"", "HYDE_THEME"), None);
    }

    #[test]
    fn an_empty_value_reads_as_missing() {
        assert_eq!(value_of("HYDE_THEME=\"\"", "HYDE_THEME"), None);
    }

    #[test]
    fn the_last_assignment_of_a_key_wins() {
        let source = "HYDE_THEME=\"Old\"\nHYDE_THEME=\"New\"\n";

        assert_eq!(value_of(source, "HYDE_THEME"), Some("New".to_owned()));
    }

    #[test]
    fn a_single_quoted_value_is_read_without_its_quotes() {
        assert_eq!(
            value_of("HYPR_SHADER='blue-light-filter'", "HYPR_SHADER"),
            Some("blue-light-filter".to_owned())
        );
    }

    #[test]
    fn a_trailing_comment_stays_out_of_an_unquoted_value() {
        assert_eq!(
            value_of("HYPR_SHADER=wallbash # the default", "HYPR_SHADER"),
            Some("wallbash".to_owned())
        );
    }

    #[test]
    fn a_line_that_assigns_nothing_is_skipped() {
        assert_eq!(assignment("case $option in"), None);
        assert_eq!(assignment("   "), None);
    }

    #[test]
    fn a_quoted_number_is_read_as_a_number() {
        assert_eq!(number::<u8>(STATERC, "enableWallDcol"), Some(1));
        assert_eq!(
            number::<f32>("BAR_FONT_SIZE=\" 10.5 \"", "BAR_FONT_SIZE"),
            Some(10.5)
        );
    }

    #[test]
    fn a_value_that_is_not_a_number_reads_as_missing() {
        assert_eq!(
            number::<f32>("BAR_FONT_SIZE=\"nil\"", "BAR_FONT_SIZE"),
            None
        );
        assert_eq!(number::<u8>("", "enableWallDcol"), None);
    }
}
