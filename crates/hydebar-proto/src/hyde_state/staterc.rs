//! Parser for the shell assignments HyDE keeps its desktop state in.
//!
//! The file is sourced by shell scripts rather than parsed by them, so it is a
//! flat list of `KEY="value"` lines. Only that shape is understood: expansions,
//! command substitutions and anything else a shell would evaluate are left
//! alone, because the bar reads the file and never runs it.

/// Values a flag is written as when HyDE means "on".
///
/// HyDE writes `1`, but the same files are edited by hand, so the spellings a
/// user would reasonably reach for are accepted as well.
const TRUTHY: [&str; 4] = ["1", "true", "yes", "on"];

/// Value assigned to `key`, with its quotes removed.
///
/// The last assignment wins, mirroring what sourcing the file in a shell would
/// leave behind: HyDE appends rather than rewrites when it records a change.
#[must_use]
pub(super) fn value_of(source: &str, key: &str) -> Option<String> {
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

/// Whether `key` is assigned a value meaning "on".
///
/// An absent key reads as off, which is what a HyDE install that never enabled
/// the feature looks like.
#[must_use]
pub(super) fn flag(source: &str, key: &str) -> bool {
    value_of(source, key)
        .map(|value| TRUTHY.contains(&value.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
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
fn is_name_char(c: char) -> bool {
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
    fn the_documented_truthy_spellings_read_as_enabled() {
        for spelling in ["1", "true", "yes", "on", "ON", "True"] {
            assert!(
                flag(&format!("enableWallDcol=\"{spelling}\""), "enableWallDcol"),
                "expected `{spelling}` to read as enabled"
            );
        }
    }

    #[test]
    fn anything_else_reads_as_disabled() {
        assert!(!flag("enableWallDcol=\"0\"", "enableWallDcol"));
        assert!(!flag("enableWallDcol=\"false\"", "enableWallDcol"));
        assert!(!flag("", "enableWallDcol"));
    }
}
