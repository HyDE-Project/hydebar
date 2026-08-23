//! Geometry of a menu box, derived from the theme and the screen it lands on.
//!
//! Nothing here is a fixed pixel count. A menu is stated in multiples of the
//! themed text size, then clamped to what the output actually offers, so the
//! same configuration lands sensibly on a laptop panel and on a large screen.

/// Preferred width of a menu, in multiples of the themed text size.
const SMALL_WIDTH_EM: f32 = 25.0;
/// Preferred width of a medium menu, in multiples of the themed text size.
const MEDIUM_WIDTH_EM: f32 = 35.0;
/// Preferred width of a large menu, in multiples of the themed text size.
const LARGE_WIDTH_EM: f32 = 45.0;
/// Narrowest a menu may become, in multiples of the themed text size.
const MIN_WIDTH_EM: f32 = 18.0;

/// Largest share of the output width a menu may take.
const MAX_WIDTH_SHARE: f32 = 0.9;
/// Largest share of the output height a menu may take.
const MAX_HEIGHT_SHARE: f32 = 0.85;

/// Gap kept between a menu and the screen edge, in multiples of the text size.
const EDGE_MARGIN_EM: f32 = 0.8;

/// The three lengths a measured window states, measured once per frame.
///
/// One struct rather than three calls on purpose: the width measurement walks
/// the whole content, and asking for it from three places made every frame
/// pay for the walk three times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuMetrics {
    /// Width the window box asks the screen for.
    pub width:      f32,
    /// Width the page inside may draw into, slack excluded.
    pub page_width: f32,
    /// Height the content needs before the screen caps it.
    pub height:     f32
}

/// How wide a menu asks to be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuSize {
    /// Room for a handful of lines.
    Small,
    /// Room for a list.
    Medium,
    /// Room for a window.
    Large,
    /// Exactly as wide as the content it holds, measured by the caller.
    Content(f32)
}

impl MenuSize {
    /// Preferred width in multiples of the themed text size.
    const fn width_em(self) -> Option<f32> {
        match self {
            Self::Small => Some(SMALL_WIDTH_EM),
            Self::Medium => Some(MEDIUM_WIDTH_EM),
            Self::Large => Some(LARGE_WIDTH_EM),
            Self::Content(_) => None
        }
    }

    /// Width of the menu box on an output `viewport_width` pixels across.
    ///
    /// A menu stated in text sizes grows with the theme; a menu stated by its
    /// content takes exactly the width its longest row needs. Either way the
    /// screen only caps the result, it never stretches it.
    #[must_use]
    pub fn width(self, font_size: f32, viewport_width: f32) -> f32 {
        let preferred = match self {
            Self::Content(content) => content,
            _ => self.width_em().unwrap_or(MEDIUM_WIDTH_EM) * font_size
        };
        let floor = MIN_WIDTH_EM * font_size;
        let ceiling = (viewport_width * MAX_WIDTH_SHARE).max(1.0);

        preferred.clamp(floor.min(ceiling), ceiling)
    }

    /// Tallest the menu box may grow on an output `viewport_height` pixels
    /// tall.
    ///
    /// Content beyond this scrolls instead of running off the screen.
    #[must_use]
    pub fn max_height(viewport_height: f32) -> f32 {
        (viewport_height * MAX_HEIGHT_SHARE).max(1.0)
    }

    /// Left offset placing a `width` wide menu under a button at `anchor_x`.
    ///
    /// The menu is centred under its button, then pushed back inside the
    /// screen: a module near either edge keeps its menu fully visible.
    #[must_use]
    pub fn left_offset(width: f32, anchor_x: f32, viewport_width: f32, font_size: f32) -> f32 {
        let margin = EDGE_MARGIN_EM * font_size;
        let rightmost = (viewport_width - width - margin).max(margin);

        (anchor_x - width / 2.0).clamp(margin, rightmost)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    #[test]
    fn the_preferred_width_scales_with_the_text_size() {
        assert_eq!(MenuSize::Medium.width(10.0, 3840.0), 350.0);
        assert_eq!(MenuSize::Medium.width(20.0, 3840.0), 700.0);
    }

    #[test]
    fn a_content_sized_menu_takes_exactly_its_content() {
        assert_eq!(MenuSize::Content(742.0).width(10.0, 3840.0), 742.0);
    }

    #[test]
    fn a_content_sized_menu_is_capped_by_the_screen() {
        let width = MenuSize::Content(5000.0).width(10.0, 1000.0);

        assert_eq!(width, 1000.0 * MAX_WIDTH_SHARE);
    }

    #[test]
    fn a_tiny_content_still_gets_a_readable_width() {
        assert_eq!(
            MenuSize::Content(10.0).width(10.0, 3840.0),
            MIN_WIDTH_EM * 10.0
        );
    }

    #[test]
    fn the_height_never_covers_the_whole_screen() {
        assert_eq!(MenuSize::max_height(1000.0), 850.0);
        assert!(MenuSize::max_height(1.0) >= 1.0);
    }

    #[test]
    fn a_menu_is_centred_under_its_button() {
        let width = 300.0;

        assert_eq!(
            MenuSize::left_offset(width, 1000.0, 1920.0, 10.0),
            1000.0 - 150.0
        );
    }

    #[test]
    fn a_menu_near_an_edge_is_pushed_back_on_screen() {
        let width = 300.0;
        let margin = EDGE_MARGIN_EM * 10.0;

        assert_eq!(MenuSize::left_offset(width, 10.0, 1920.0, 10.0), margin);
        assert_eq!(
            MenuSize::left_offset(width, 1910.0, 1920.0, 10.0),
            1920.0 - width - margin
        );
    }

    #[test]
    fn a_menu_wider_than_the_screen_still_starts_on_screen() {
        let offset = MenuSize::left_offset(2000.0, 100.0, 800.0, 10.0);

        assert!(offset >= 0.0);
    }
}
