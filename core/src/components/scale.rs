//! The one place a hand written size is turned into a scaled one.
//!
//! A widget that states its own size in pixels, a heading twice the body text
//! for instance, would ignore the magnification the screen calls for and stay
//! small while everything around it grows. Stating the same size through
//! [`scaled`] keeps the intent, twice the body text, and lets it follow the
//! magnification the whole bar is drawn at.

use std::sync::atomic::{AtomicU32, Ordering};

use hydebar_proto::config::DEFAULT_FONT_SIZE;

/// Text size the bar renders with, stored as bits so it can live in an atomic.
///
/// Zero means it was never declared, which reads back as the built in size.
static BASE: AtomicU32 = AtomicU32::new(0);

/// Declares the text size every other size is stated against.
///
/// This is the single source every size in the bar derives from: the themed
/// text size, magnified for the screen. Called once at startup and again on
/// every configuration reload, so a size change reaches every widget at once.
pub fn set_base(size: f32) {
    if size.is_finite() && size > 0.0 {
        BASE.store(size.to_bits(), Ordering::Relaxed);
    }
}

/// Text size in force, the built in one until [`set_base`] says otherwise.
#[must_use]
pub fn base() -> f32 {
    match BASE.load(Ordering::Relaxed) {
        0 => DEFAULT_FONT_SIZE,
        bits => f32::from_bits(bits)
    }
}

/// Magnification the bar is rendered at, relative to the built in size.
#[must_use]
pub fn factor() -> f32 {
    base() / DEFAULT_FONT_SIZE
}

/// Magnification the screen asked for, stored as bits.
///
/// Distinct from [`factor`]: that one says how much larger the text is than the
/// built in size, this one says how much the configured sizes were multiplied
/// by for this screen, and it is what a configuration read from disk has to be
/// multiplied by again.
static SCREEN: AtomicU32 = AtomicU32::new(0);

/// Declares the magnification the screen asked for.
pub fn set_screen_factor(factor: f32) {
    if factor.is_finite() && factor > 1.0 {
        SCREEN.store(factor.to_bits(), Ordering::Relaxed);
    }
}

/// Magnification the screen asked for, one until it is declared.
#[must_use]
pub fn screen_factor() -> f32 {
    match SCREEN.load(Ordering::Relaxed) {
        0 => 1.0,
        bits => f32::from_bits(bits)
    }
}

/// Turns a size written against the built in text size into the size to use.
///
/// A `12` written when the text size was the built in one keeps its proportion
/// to the body text whatever the base becomes.
#[must_use]
pub fn scaled(size: f32) -> f32 {
    size * factor()
}

/// Gap between an icon and the value it labels, in pixels.
///
/// The same law the module wrappers use, restated here for the views that
/// have no appearance at hand: one number, or the same pair reads tighter in
/// one module than in the next.
#[must_use]
pub fn icon_gap() -> f32 {
    base() * hydebar_proto::config::ICON_LABEL_GAP_EM
}

/// Gap between two readouts standing side by side inside one module.
///
/// Exactly the gap two neighbouring modules show between them — the side
/// padding of each, twice — so a module drawing several readouts is
/// indistinguishable from several modules standing next to each other.
#[must_use]
pub fn item_gap() -> f32 {
    base() * (hydebar_proto::config::MODULE_SIDE_PADDING_EM * 2.0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::sync::{Mutex, MutexGuard, PoisonError};

    use super::*;

    /// Serialises the tests: they all share the one declared base, and running
    /// them side by side would let one clear what another just declared.
    static GUARD: Mutex<()> = Mutex::new(());

    fn alone() -> MutexGuard<'static, ()> {
        let guard = GUARD.lock().unwrap_or_else(PoisonError::into_inner);
        forget();
        SCREEN.store(0, Ordering::Relaxed);
        guard
    }

    fn forget() {
        BASE.store(0, Ordering::Relaxed);
    }

    #[test]
    fn without_a_base_the_built_in_size_is_used() {
        let _alone = alone();

        assert_eq!(base(), DEFAULT_FONT_SIZE);
        assert_eq!(factor(), 1.0);
        assert_eq!(scaled(12.0), 12.0);
    }

    #[test]
    fn a_doubled_base_doubles_every_size() {
        let _alone = alone();
        set_base(DEFAULT_FONT_SIZE * 2.0);

        assert_eq!(factor(), 2.0);
        assert_eq!(scaled(12.0), 24.0);

        forget();
    }

    #[test]
    fn the_proportion_between_two_sizes_survives() {
        let _alone = alone();
        set_base(DEFAULT_FONT_SIZE * 1.5);

        assert_eq!(scaled(20.0) / scaled(10.0), 2.0);

        forget();
    }

    #[test]
    fn the_screen_factor_is_kept_apart_from_the_text_size() {
        let _alone = alone();
        set_base(DEFAULT_FONT_SIZE * 2.0);

        assert_eq!(screen_factor(), 1.0);

        set_screen_factor(2.0);
        assert_eq!(screen_factor(), 2.0);

        SCREEN.store(0, Ordering::Relaxed);
        forget();
    }

    #[test]
    fn a_screen_factor_that_would_shrink_the_bar_is_refused() {
        let _alone = alone();
        set_screen_factor(0.5);

        assert_eq!(screen_factor(), 1.0);
    }

    #[test]
    fn a_nonsense_base_is_refused() {
        let _alone = alone();
        set_base(f32::NAN);
        set_base(-3.0);
        set_base(0.0);

        assert_eq!(base(), DEFAULT_FONT_SIZE);
    }
}
