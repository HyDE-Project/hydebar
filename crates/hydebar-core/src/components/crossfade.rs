//! A value that dissolves into its replacement instead of snapping.
//!
//! A readout on the bar changes while the user is looking straight at it —
//! that is what a readout is for — and a hard swap is the most abrupt motion
//! the bar makes. Holding the outgoing text a moment and fading the incoming
//! one over it turns the change into the same family of motion as every other
//! transition on the bar.

use std::time::Duration;

use iced::{
    Element, Theme,
    widget::{stack, text}
};

use crate::animation::{STANDARD, Spring};

/// A displayed string and the one it is dissolving out of.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use hydebar_core::components::crossfade::Crossfade;
///
/// let mut value = Crossfade::default();
/// value.set("12:00".to_owned(), true);
/// assert!(!value.is_animating(), "the first value simply appears");
///
/// value.set("12:01".to_owned(), true);
/// assert!(value.is_animating());
///
/// while value.advance(Duration::from_millis(8)) {}
/// assert_eq!(value.current(), "12:01");
/// ```
#[derive(Debug, Clone)]
pub struct Crossfade {
    previous: Option<String>,
    current:  String,
    progress: Spring
}

impl Default for Crossfade {
    fn default() -> Self {
        Self {
            previous: None,
            current:  String::new(),
            progress: Spring::new(1.0).with_response(STANDARD)
        }
    }
}

impl Crossfade {
    /// The value on display, the one a change dissolves towards.
    #[must_use]
    pub fn current(&self) -> &str {
        &self.current
    }

    /// Replaces the value, dissolving unless fades are off.
    ///
    /// The very first value appears outright — there is nothing on screen to
    /// dissolve from, and a bar fading its readouts in at startup would fight
    /// the entrance wave that already carries them.
    pub fn set(&mut self, value: String, animated: bool) {
        if value == self.current {
            return;
        }

        if !animated || self.current.is_empty() {
            self.previous = None;
            self.current = value;
            self.progress.snap_to(1.0);
            return;
        }

        self.previous = Some(std::mem::replace(&mut self.current, value));
        self.progress.snap_to(0.0);
        self.progress.set_target(1.0);
    }

    /// Advances the dissolve and reports whether it still needs frames.
    pub fn advance(&mut self, elapsed: Duration) -> bool {
        let running = self.progress.advance(elapsed);

        if !running {
            self.previous = None;
        }

        running
    }

    /// Whether the dissolve still needs frames.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.progress.is_animating()
    }

    /// Renders the value, the outgoing text still fading under the incoming.
    ///
    /// Owned on purpose: module views hand back `'static` elements, and two
    /// short strings are cheaper than a lifetime through every module.
    #[must_use]
    pub fn element<M: 'static>(&self, size: f32) -> Element<'static, M> {
        let share = self.progress.value().clamp(0.0, 1.0);

        let incoming = text(self.current.clone())
            .size(size)
            .style(move |theme: &Theme| text::Style {
                color: Some(theme.palette().text.scale_alpha(share))
            });

        match &self.previous {
            Some(previous) if share < 1.0 => stack![
                text(previous.clone())
                    .size(size)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(theme.palette().text.scale_alpha(1.0 - share))
                    }),
                incoming
            ]
            .into(),
            _ => text(self.current.clone())
                .size(size)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.palette().text)
                })
                .into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain(value: &mut Crossfade) {
        let mut frames = 0;
        while value.advance(Duration::from_millis(8)) {
            frames += 1;
            assert!(frames < 1000, "crossfade failed to settle");
        }
    }

    #[test]
    fn the_first_value_appears_without_a_dissolve() {
        let mut value = Crossfade::default();

        value.set("12:00".to_owned(), true);

        assert_eq!(value.current(), "12:00");
        assert!(!value.is_animating());
    }

    #[test]
    fn a_change_dissolves_and_settles_on_the_new_value() {
        let mut value = Crossfade::default();
        value.set("12:00".to_owned(), true);

        value.set("12:01".to_owned(), true);

        assert!(value.is_animating());
        assert_eq!(value.current(), "12:01");

        drain(&mut value);

        assert!(!value.is_animating());
        assert!(value.previous.is_none(), "the outgoing text is let go");
    }

    #[test]
    fn the_same_value_again_does_not_restart_the_dissolve() {
        let mut value = Crossfade::default();
        value.set("12:00".to_owned(), true);
        value.set("12:00".to_owned(), true);

        assert!(!value.is_animating());
    }

    #[test]
    fn a_change_mid_dissolve_hands_over_to_the_newest_value() {
        let mut value = Crossfade::default();
        value.set("10%".to_owned(), true);
        value.set("11%".to_owned(), true);
        let _ = value.advance(Duration::from_millis(30));

        value.set("12%".to_owned(), true);

        assert_eq!(value.current(), "12%");

        drain(&mut value);

        assert_eq!(value.current(), "12%");
    }

    #[test]
    fn disabled_fades_swap_outright() {
        let mut value = Crossfade::default();
        value.set("12:00".to_owned(), false);
        value.set("12:01".to_owned(), false);

        assert_eq!(value.current(), "12:01");
        assert!(!value.is_animating());
    }
}
