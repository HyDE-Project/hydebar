//! Look the compositor is configured with, read so the bar can match it.
//!
//! A bar that rounds its islands differently from the windows beside them, or
//! keeps animating while the session has animations switched off, reads as a
//! foreign thing on the desktop. Everything here is what the compositor already
//! knows about itself, so nothing has to be configured twice.

mod answers;
mod query;

use std::{
    sync::{Mutex, PoisonError},
    time::{Duration, Instant}
};

/// How long one reading of the look keeps answering for the compositor.
///
/// A single theme reload asks for the look more than once — the watcher reads
/// it for the theme and the window re-reads it to adopt the screen — and each
/// uncached reading costs a round trip to the compositor. The window is a
/// backstop: the compositor announces a configuration reload on its event
/// socket and [`CompositorLook::invalidate`] answers it, so an edit lands on
/// the next read and the clock only covers changes the socket never
/// announced, such as a keyword set by hand.
const FRESH_FOR: Duration = Duration::from_secs(30);

/// The last look read, and when it was read.
static LAST_READ: Mutex<Option<(Instant, CompositorLook)>> = Mutex::new(None);

/// Look the compositor draws its own windows with.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CompositorLook {
    /// Corner radius of a window, in pixels.
    pub rounding:     Option<f32>,
    /// Gap kept outside a window, in pixels.
    pub gaps_out:     Option<f32>,
    /// Gap kept between windows, in pixels.
    pub gaps_in:      Option<f32>,
    /// Whether the session animates at all.
    pub animations:   Option<bool>,
    /// Whether the compositor blurs at all.
    pub blur:         Option<bool>,
    /// Width of the border around a window, in pixels.
    pub border_width: Option<f32>,
    /// Leading colour of the active window border, as RGBA in unit range.
    pub border_color: Option<[f32; 4]>,
    /// Whether windows cast a shadow at all.
    pub shadow:       Option<bool>,
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

        let look = query::read();
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
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

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
