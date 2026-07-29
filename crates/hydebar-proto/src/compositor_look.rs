//! Look the compositor is configured with, read so the bar can match it.
//!
//! A bar that rounds its islands differently from the windows beside them, or
//! keeps animating while the session has animations switched off, reads as a
//! foreign thing on the desktop. Everything here is what the compositor already
//! knows about itself, so nothing has to be configured twice.

use std::process::Command;

/// Look the compositor draws its own windows with.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CompositorLook {
    /// Corner radius of a window, in pixels.
    pub rounding:   Option<f32>,
    /// Gap kept outside a window, in pixels.
    pub gaps_out:   Option<f32>,
    /// Gap kept between windows, in pixels.
    pub gaps_in:    Option<f32>,
    /// Whether the session animates at all.
    pub animations: Option<bool>
}

impl CompositorLook {
    /// Reads the look from the compositor.
    ///
    /// Anything the compositor does not answer for is left unset, and the bar
    /// keeps whatever its own configuration says.
    #[must_use]
    pub fn read() -> Self {
        Self {
            rounding:   query("decoration:rounding").and_then(|answer| parse_number(&answer)),
            gaps_out:   query("general:gaps_out").and_then(|answer| parse_gap(&answer)),
            gaps_in:    query("general:gaps_in").and_then(|answer| parse_gap(&answer)),
            animations: query("animations:enabled").and_then(|answer| parse_flag(&answer))
        }
    }
}

/// Asks the compositor for one option.
fn query(option: &str) -> Option<String> {
    let output = Command::new("hyprctl")
        .args(["getoption", option])
        .output()
        .ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Reads a number out of an answer like `int: 3` or `float: 0.9`.
#[must_use]
pub fn parse_number(answer: &str) -> Option<f32> {
    answer
        .lines()
        .find_map(|line| line.split_once(':'))
        .and_then(|(kind, value)| {
            matches!(kind.trim(), "int" | "float").then(|| value.trim().parse().ok())?
        })
}

/// Reads the first gap out of an answer like `css gap data: 8 8 8 8`.
///
/// The compositor states gaps the way a stylesheet does, four sides at once;
/// the bar only ever needs one number, and the sides are equal in every
/// configuration that looks deliberate.
#[must_use]
pub fn parse_gap(answer: &str) -> Option<f32> {
    answer
        .lines()
        .find_map(|line| line.split_once(':'))
        .and_then(|(kind, value)| {
            kind.trim()
                .ends_with("gap data")
                .then(|| value.split_whitespace().next()?.parse().ok())?
        })
}

/// Reads a flag out of an answer like `bool: true`.
#[must_use]
pub fn parse_flag(answer: &str) -> Option<bool> {
    answer
        .lines()
        .find_map(|line| line.split_once(':'))
        .and_then(|(kind, value)| match (kind.trim(), value.trim()) {
            ("bool", "true") => Some(true),
            ("bool", "false") => Some(false),
            _ => None
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whole_number_is_read() {
        assert_eq!(parse_number("int: 3\nset: true\n"), Some(3.0));
    }

    #[test]
    fn a_fractional_number_is_read() {
        assert_eq!(parse_number("float: 0.900000\nset: true\n"), Some(0.9));
    }

    #[test]
    fn a_gap_takes_the_first_of_the_four_sides() {
        assert_eq!(parse_gap("css gap data: 8 8 8 8\nset: true\n"), Some(8.0));
    }

    #[test]
    fn a_flag_is_read_both_ways() {
        assert_eq!(parse_flag("bool: true\nset: false\n"), Some(true));
        assert_eq!(parse_flag("bool: false\nset: true\n"), Some(false));
    }

    #[test]
    fn an_answer_of_another_kind_is_refused() {
        assert_eq!(parse_number("bool: true"), None);
        assert_eq!(parse_gap("int: 3"), None);
        assert_eq!(parse_flag("int: 3"), None);
    }

    #[test]
    fn an_unknown_option_is_reported_as_nothing() {
        assert_eq!(parse_number("no such option"), None);
        assert_eq!(parse_gap("no such option"), None);
        assert_eq!(parse_flag("no such option"), None);
    }

    #[test]
    fn an_unread_look_changes_nothing() {
        let look = CompositorLook::default();

        assert_eq!(look.rounding, None);
        assert_eq!(look.gaps_out, None);
        assert_eq!(look.gaps_in, None);
        assert_eq!(look.animations, None);
    }
}
