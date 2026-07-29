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
/// Preferred width of a wide menu, in multiples of the themed text size.
///
/// Used by the windows that list things side by side, the module editor above
/// all, where three columns have to fit next to each other.
const WIDE_WIDTH_EM: f32 = 70.0;

/// Narrowest a menu may become, in multiples of the themed text size.
const MIN_WIDTH_EM: f32 = 18.0;

/// Largest share of the output width a menu may take.
const MAX_WIDTH_SHARE: f32 = 0.9;
/// Largest share of the output height a menu may take.
const MAX_HEIGHT_SHARE: f32 = 0.85;

/// Gap kept between a menu and the screen edge, in multiples of the text size.
const EDGE_MARGIN_EM: f32 = 0.8;

/// How wide a menu asks to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSize {
    Small,
    Medium,
    Large,
    /// As wide as the output comfortably allows.
    Wide
}

impl MenuSize {
    /// Preferred width in multiples of the themed text size.
    const fn width_em(self) -> f32 {
        match self {
            Self::Small => SMALL_WIDTH_EM,
            Self::Medium => MEDIUM_WIDTH_EM,
            Self::Large => LARGE_WIDTH_EM,
            Self::Wide => WIDE_WIDTH_EM
        }
    }

    /// Width of the menu box on an output `viewport_width` pixels across.
    ///
    /// The preferred width scales with `font_size`, so a larger theme yields a
    /// larger menu, and never outgrows the share of the screen a menu is
    /// allowed to cover.
    #[must_use]
    pub fn width(self, font_size: f32, viewport_width: f32) -> f32 {
        let preferred = self.width_em() * font_size;
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
mod tests {
    use super::*;

    #[test]
    fn the_preferred_width_scales_with_the_text_size() {
        assert_eq!(MenuSize::Medium.width(10.0, 3840.0), 350.0);
        assert_eq!(MenuSize::Medium.width(20.0, 3840.0), 700.0);
    }

    #[test]
    fn a_narrow_screen_caps_the_width() {
        let width = MenuSize::Wide.width(10.0, 600.0);

        assert!(width <= 600.0 * MAX_WIDTH_SHARE);
        assert_eq!(width, 540.0);
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
