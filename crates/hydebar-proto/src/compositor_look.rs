//! Look the compositor is configured with, read so the bar can match it.
//!
//! A bar that rounds its islands differently from the windows beside them, or
//! keeps animating while the session has animations switched off, reads as a
//! foreign thing on the desktop. Everything here is what the compositor already
//! knows about itself, so nothing has to be configured twice.

use std::{
    process::Command,
    sync::{Mutex, PoisonError},
    time::{Duration, Instant}
};

/// How long one reading of the look keeps answering for the compositor.
///
/// A single theme reload asks for the look more than once — the watcher reads
/// it for the theme and the window re-reads it to adopt the screen — and each
/// uncached reading costs four spawned processes. The window is a backstop:
/// the compositor announces a configuration reload on its event socket and
/// [`CompositorLook::invalidate`] answers it, so an edit lands on the next
/// read and the clock only covers changes the socket never announced, such as
/// a keyword set by hand.
const FRESH_FOR: Duration = Duration::from_secs(30);

/// The last look read, and when it was read.
static LAST_READ: Mutex<Option<(Instant, CompositorLook)>> = Mutex::new(None);

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
    pub animations: Option<bool>,
    /// Whether the compositor blurs at all.
    pub blur:       Option<bool>,
    /// Width of the border around a window, in pixels.
    pub border_width: Option<f32>,
    /// Leading colour of the active window border, as RGBA in unit range.
    pub border_color: Option<[f32; 4]>,
    /// Whether windows cast a shadow at all.
    pub shadow: Option<bool>,
    /// Reach of the shadow, in pixels.
    pub shadow_range: Option<f32>,
    /// Colour of the shadow, as RGBA in unit range.
    pub shadow_color: Option<[f32; 4]>
}

impl CompositorLook {
    /// Reads the look from the compositor.
    ///
    /// Anything the compositor does not answer for is left unset, and the bar
    /// keeps whatever its own configuration says. A reading taken within
    /// [`FRESH_FOR`] answers again instead of asking the compositor over.
    #[must_use]
    pub fn read() -> Self {
        {
            let last = LAST_READ.lock().unwrap_or_else(PoisonError::into_inner);

            if let Some((at, look)) = *last
                && at.elapsed() < FRESH_FOR
            {
                return look;
            }
        }

        let look = Self::query_compositor();
        *LAST_READ.lock().unwrap_or_else(PoisonError::into_inner) = Some((Instant::now(), look));

        look
    }

    /// Forgets the cached reading, so the next read asks the compositor.
    ///
    /// Called when the compositor announces a configuration reload: the look
    /// may have changed with it, and waiting out the freshness window would
    /// leave the bar drawn against the old one.
    pub fn invalidate() {
        *LAST_READ.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    /// Asks the compositor for every part of the look.
    fn query_compositor() -> Self {
        Self {
            rounding:   query("decoration:rounding").and_then(|answer| parse_number(&answer)),
            gaps_out:   query("general:gaps_out").and_then(|answer| parse_gap(&answer)),
            gaps_in:    query("general:gaps_in").and_then(|answer| parse_gap(&answer)),
            animations: query("animations:enabled").and_then(|answer| parse_flag(&answer)),
            blur:       query("decoration:blur:enabled").and_then(|answer| parse_flag(&answer)),
            border_width: query("general:border_size").and_then(|answer| parse_number(&answer)),
            border_color: query("general:col.active_border")
                .and_then(|answer| parse_gradient_color(&answer)),
            shadow: query("decoration:shadow:enabled").and_then(|answer| parse_flag(&answer)),
            shadow_range: query("decoration:shadow:range")
                .and_then(|answer| parse_number(&answer)),
            shadow_color: query("decoration:shadow:color")
                .and_then(|answer| parse_gradient_color(&answer))
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

/// Reads the leading colour out of an answer like
/// `gradient data: f2e0c8a4 f2524129 45deg`.
///
/// The compositor paints the border as a gradient; one border colour is all
/// an island needs, and the leading stop is the one the eye reads at the
/// corner the gradient starts from. The spelling is AARRGGBB.
#[must_use]
pub fn parse_gradient_color(answer: &str) -> Option<[f32; 4]> {
    let value = answer
        .lines()
        .find_map(|line| line.split_once(':'))
        .filter(|(kind, _)| kind.trim().ends_with("gradient data"))
        .map(|(_, value)| value)?;

    let token = value.split_whitespace().next()?;

    if token.len() != 8 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    let channel = |from: usize| {
        u8::from_str_radix(&token[from..from + 2], 16)
            .map(|byte| f32::from(byte) / 255.0)
            .ok()
    };

    Some([channel(2)?, channel(4)?, channel(6)?, channel(0)?])
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

    /// One test rather than two: the reading shares one process-wide slot, and
    /// two tests racing over it would see each other's entries.
    #[test]
    fn a_fresh_reading_answers_again_and_a_stale_one_is_retaken() {
        let sentinel = CompositorLook {
            rounding: Some(42.0),
            ..CompositorLook::default()
        };

        *LAST_READ.lock().expect("lock") = Some((Instant::now(), sentinel));
        assert_eq!(CompositorLook::read(), sentinel);

        let long_ago = Instant::now()
            .checked_sub(FRESH_FOR * 2)
            .expect("the clock reaches back beyond the freshness window");
        *LAST_READ.lock().expect("lock") = Some((long_ago, sentinel));
        let _ = CompositorLook::read();

        let (taken_at, _) = LAST_READ
            .lock()
            .expect("lock")
            .expect("a reading was just recorded");
        assert!(
            taken_at.elapsed() < FRESH_FOR,
            "a stale reading must be retaken from the compositor"
        );
    }
}
