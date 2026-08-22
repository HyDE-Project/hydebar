//! Where on each screen the bar's own strip was put.
//!
//! A layer surface never learns its own place. The protocol hands a client the
//! width and the height it was given and nothing else, so a bar drawing a
//! second surface over the whole screen cannot work out from its own numbers
//! where on that screen the strip stands: what pushes it down is the space
//! every other layer above it reserved, and that is somebody else's business.
//!
//! It matters to exactly one thing. The canvas covers the screen from its top
//! edge, and the modules leave the strip by travelling out of it, so the row
//! they leave has to be the row the strip is really on. Taken as the top of
//! the screen, every block of the bar jumped the height of whatever stands
//! above it before it started moving — a panel of its own above the bar is
//! ordinary on a desktop, and the bar leapt into it on the first frame.
//!
//! So the compositor is asked, the same way it is already asked for the
//! rounding, the gaps and the blur it draws its own windows with, and with a
//! cache of the same shape: the answer only changes when a layer above the bar
//! is mapped or unmapped.

use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
    time::{Duration, Instant}
};

use super::{scaling::compositor_answer, wayland::MAIN_NAMESPACE};

/// How long one reading keeps answering for the compositor.
///
/// Short enough that a panel started beside the bar is noticed by the next
/// unfolding, long enough that the burst of questions one workspace change
/// brings costs a single round trip.
const FRESH_FOR: Duration = Duration::from_secs(2);

/// Longest the compositor gets to answer.
///
/// A busy compositor must cost the bar a stale row, not a stalled unfolding.
const TIMEOUT: Duration = Duration::from_millis(400);

/// The last reading, and the moment it was read.
static LAST_READ: Mutex<Option<(Instant, HashMap<String, f32>)>> = Mutex::new(None);

/// The row the strip stands on, screen by screen.
///
/// Keyed by the name of the screen, in the same logical pixels the surfaces
/// are drawn in. A screen the bar does not draw on is absent rather than zero,
/// so a caller can tell a strip at the very top from one it knows nothing
/// about.
#[must_use]
pub async fn strip_rows() -> HashMap<String, f32> {
    let now = Instant::now();

    if let Some((read, rows)) = LAST_READ
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        && now.duration_since(*read) < FRESH_FOR
    {
        return rows.clone();
    }

    let Some(answer) = tokio::time::timeout(TIMEOUT, compositor_answer("j/layers"))
        .await
        .ok()
        .flatten()
    else {
        return remembered();
    };

    let rows = parse_rows(&answer);

    if rows.is_empty() {
        return remembered();
    }

    *LAST_READ.lock().unwrap_or_else(PoisonError::into_inner) = Some((now, rows.clone()));

    rows
}

/// Whatever was read last, however long ago.
///
/// A compositor that did not answer has not moved the bar; the row it gave
/// last is a better answer than the top of the screen.
fn remembered() -> HashMap<String, f32> {
    LAST_READ
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map(|(_, rows)| rows.clone())
        .unwrap_or_default()
}

/// Picks the bar's own surface out of the compositor's answer, screen by
/// screen.
///
/// The answer is keyed by screen and holds one list per stacking level; the
/// bar's strip is the surface carrying its own namespace, wherever the
/// compositor happens to have stacked it.
fn parse_rows(json: &str) -> HashMap<String, f32> {
    let Ok(screens) = serde_json::from_str::<serde_json::Value>(json) else {
        return HashMap::new();
    };

    let Some(screens) = screens.as_object() else {
        return HashMap::new();
    };

    screens
        .iter()
        .filter_map(|(screen, levels)| {
            let row = levels
                .get("levels")?
                .as_object()?
                .values()
                .filter_map(serde_json::Value::as_array)
                .flatten()
                .find(|layer| layer["namespace"].as_str() == Some(MAIN_NAMESPACE))?
                .get("y")?
                .as_f64()?;

            #[expect(
                clippy::cast_possible_truncation,
                reason = "a screen row is a small pixel count"
            )]
            Some((screen.clone(), row as f32))
        })
        .collect()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    const ANSWER: &str = r#"{
        "HDMI-A-1": { "levels": {
            "0": [ { "namespace": "hydebar-desk-layer", "x": 0, "y": 0, "w": 3840, "h": 2160 } ],
            "2": [ { "namespace": "some-other-panel", "x": 0, "y": 0, "w": 3840, "h": 38 },
                   { "namespace": "hydebar-main-layer", "x": 0, "y": 38, "w": 3840, "h": 76 } ]
        } },
        "eDP-1": { "levels": {
            "2": [ { "namespace": "hydebar-main-layer", "x": 0, "y": 0, "w": 1920, "h": 38 } ]
        } }
    }"#;

    #[test]
    fn the_strip_is_read_from_under_whatever_reserved_the_top() {
        let rows = parse_rows(ANSWER);

        assert_eq!(rows.get("HDMI-A-1"), Some(&38.0));
    }

    #[test]
    fn a_strip_with_nothing_above_it_stands_at_the_top() {
        let rows = parse_rows(ANSWER);

        assert_eq!(rows.get("eDP-1"), Some(&0.0));
    }

    #[test]
    fn a_screen_the_bar_does_not_draw_on_is_left_out() {
        let rows = parse_rows(
            r#"{ "DP-2": { "levels": { "2": [
                { "namespace": "some-other-panel", "x": 0, "y": 0, "w": 100, "h": 38 }
            ] } } }"#
        );

        assert!(rows.is_empty());
    }

    #[test]
    fn an_answer_that_is_not_the_compositors_reads_as_nothing() {
        assert!(parse_rows("not json at all").is_empty());
        assert!(parse_rows("[]").is_empty());
    }

    #[test]
    fn the_strip_is_found_whatever_level_it_was_stacked_on() {
        let rows = parse_rows(
            r#"{ "DP-2": { "levels": { "1": [
                { "namespace": "hydebar-main-layer", "x": 0, "y": 12, "w": 100, "h": 38 }
            ] } } }"#
        );

        assert_eq!(rows.get("DP-2"), Some(&12.0));
    }
}
