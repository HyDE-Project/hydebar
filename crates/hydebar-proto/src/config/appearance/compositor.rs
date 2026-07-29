//! Folding the compositor look into the bar appearance.
//!
//! Only what the user left unset is taken: a bar told to round its islands by
//! ten keeps rounding them by ten, while a bar that says nothing follows the
//! windows around it.

use crate::{compositor_look::CompositorLook, config::Appearance};

impl Appearance {
    /// Takes the look of the compositor for everything the configuration and
    /// the theme left unsaid.
    ///
    /// Animations are the exception: a session with animations switched off
    /// wants the bar still, whatever the bar was told, since the whole point of
    /// that switch is a desktop that does not move.
    pub fn follow_compositor(&mut self, look: &CompositorLook) {
        if self.radius.is_none() {
            self.radius = look.rounding;
        }

        if self.side_padding.is_none() {
            self.side_padding = look
                .gaps_out
                .map(|gap| unscaled_padding(gap, self.scale_factor));
        }

        if let Some(false) = look.animations {
            self.animations.enabled = false;
        }
    }
}

/// Restates a compositor gap in the pixels the bar lays itself out in.
///
/// The bar hands its widgets to a renderer running at
/// [`Appearance::scale_factor`], which multiplies every length again before it
/// reaches the screen. A gap the compositor measured in screen pixels would
/// therefore land twice as far in if it were passed through untouched, so it is
/// divided out here and the renderer puts it back.
fn unscaled_padding(gap: f32, scale_factor: f64) -> f32 {
    if scale_factor <= 0.0 {
        return gap;
    }

    (f64::from(gap) / scale_factor) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> CompositorLook {
        CompositorLook {
            rounding:   Some(3.0),
            gaps_out:   Some(8.0),
            gaps_in:    Some(3.0),
            animations: Some(true)
        }
    }

    #[test]
    fn an_unset_radius_follows_the_windows() {
        let mut appearance = Appearance {
            radius: None,
            ..Appearance::default()
        };

        appearance.follow_compositor(&look());

        assert_eq!(appearance.radius, Some(3.0));
    }

    #[test]
    fn a_configured_radius_is_kept() {
        let mut appearance = Appearance {
            radius: Some(10.0),
            ..Appearance::default()
        };

        appearance.follow_compositor(&look());

        assert_eq!(appearance.radius, Some(10.0));
    }

    #[test]
    fn a_still_session_stills_the_bar() {
        let mut appearance = Appearance::default();
        appearance.animations.enabled = true;

        appearance.follow_compositor(&CompositorLook {
            animations: Some(false),
            ..look()
        });

        assert!(!appearance.animations.enabled);
    }

    #[test]
    fn an_animated_session_leaves_the_choice_to_the_bar() {
        let mut appearance = Appearance::default();
        appearance.animations.enabled = false;

        appearance.follow_compositor(&look());

        assert!(!appearance.animations.enabled);
    }

    #[test]
    fn a_compositor_that_says_nothing_changes_nothing() {
        let mut appearance = Appearance {
            radius: None,
            side_padding: None,
            ..Appearance::default()
        };
        let before = appearance.animations.enabled;
        let padding_before = appearance.bar_padding();

        appearance.follow_compositor(&CompositorLook::default());

        assert_eq!(appearance.radius, None);
        assert_eq!(appearance.side_padding, None);
        assert_eq!(appearance.bar_padding(), padding_before);
        assert_eq!(appearance.animations.enabled, before);
    }

    #[test]
    fn an_unset_side_padding_follows_the_window_gaps() {
        let mut appearance = Appearance {
            side_padding: None,
            ..Appearance::default()
        };

        appearance.follow_compositor(&look());

        assert_eq!(appearance.side_padding, Some(8.0));
        assert_eq!(appearance.bar_padding()[1], 8.0);
    }

    #[test]
    fn a_configured_side_padding_is_kept() {
        let mut appearance = Appearance {
            side_padding: Some(20.0),
            ..Appearance::default()
        };

        appearance.follow_compositor(&look());

        assert_eq!(appearance.side_padding, Some(20.0));
    }

    #[test]
    fn a_scaled_bar_states_the_window_gap_once() {
        let mut appearance = Appearance {
            side_padding: None,
            scale_factor: 2.0,
            ..Appearance::default()
        };

        appearance.follow_compositor(&look());

        assert_eq!(appearance.side_padding, Some(4.0));
    }

    #[test]
    fn a_bar_without_a_scale_keeps_the_gap_verbatim() {
        assert_eq!(unscaled_padding(8.0, 0.0), 8.0);
        assert_eq!(unscaled_padding(8.0, 1.0), 8.0);
    }
}
