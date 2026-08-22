//! Reading a configured colour as the colour the renderer paints with.
//!
//! The configuration and the `HyDE` theme both state colours as
//! [`Rgba`] — plain channels, with no idea a renderer exists. This is the one
//! place that turns them into the toolkit's own colour, so the schema crate
//! stays free of the toolkit and every widget reads its shades through the
//! same conversion.

use hydebar_proto::theme_source::Rgba;
use iced::{Color, theme::palette};

/// The renderer's colour for a value the configuration states.
#[must_use]
pub const fn painted(color: Rgba) -> Color {
    Color {
        a: color.a,
        ..Color::from_rgb8(color.r, color.g, color.b)
    }
}

/// Pairs a surface with the text that has to stay readable on it.
///
/// The pair is built by the toolkit, which lightens or darkens the text until
/// it reads against the surface; an entry that names no text of its own is
/// given `fallback` to start from.
#[must_use]
pub fn readable_pair(surface: Rgba, text: Option<Rgba>, fallback: Color) -> palette::Pair {
    palette::Pair::new(painted(surface), text.map_or(fallback, painted))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{Rgba, painted, readable_pair};

    #[test]
    fn a_stated_colour_keeps_its_channels_and_its_alpha() {
        let color = painted(Rgba::rgba(10, 20, 30, 0.5));

        assert!((color.r - 10.0 / 255.0).abs() < f32::EPSILON);
        assert!((color.g - 20.0 / 255.0).abs() < f32::EPSILON);
        assert!((color.b - 30.0 / 255.0).abs() < f32::EPSILON);
        assert!((color.a - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn an_entry_naming_no_text_starts_from_the_fallback() {
        let fallback = painted(Rgba::rgb(255, 255, 255));
        let pair = readable_pair(Rgba::rgb(1, 2, 3), None, fallback);

        assert_eq!(pair.color, painted(Rgba::rgb(1, 2, 3)));
        assert_eq!(pair.text, fallback);
    }

    #[test]
    fn an_entry_naming_a_readable_text_keeps_it() {
        let fallback = painted(Rgba::rgb(200, 200, 200));
        let pair = readable_pair(Rgba::rgb(255, 255, 255), Some(Rgba::rgb(0, 0, 0)), fallback);

        assert_eq!(pair.text, painted(Rgba::rgb(0, 0, 0)));
    }
}
