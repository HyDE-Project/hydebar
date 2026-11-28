//! Pure placement geometry for a bar-attached drawer menu.
//!
//! The math is kept free of GUI types so it can be tested without a window.
//! It answers where a menu hangs from the bar, how much room it has, and how
//! far it has slid out during the open animation. The geometry is modelled
//! after Mandara's joined drawer: the drawer hangs from the bar edge with a
//! zero gap, keeps clear of screen edges, and degrades to the available room
//! instead of overflowing the viewport.

use hydebar_proto::config::Position;

/// Largest share of the output width a menu may take.
const MAX_WIDTH_SHARE: f32 = 0.9;
/// Largest share of the output height a menu may take.
const MAX_HEIGHT_SHARE: f32 = 0.85;
/// Narrowest a menu may become, in multiples of the themed text size.
const MIN_WIDTH_EM: f32 = 18.0;
/// Gap kept between a menu and the screen edge, in multiples of the text size.
const EDGE_MARGIN_EM: f32 = 0.8;
/// How far the drawer travels from the bar edge while opening, in text sizes.
const SLIDE_EM: f32 = 0.8;

/// Inputs required to place a drawer menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementInput {
    /// Screen edge the bar is docked to.
    pub bar_position:     Position,
    /// Anchor point of the button that opened the menu, in surface coordinates.
    pub anchor_x:         f32,
    /// Anchor point of the button that opened the menu, in surface coordinates.
    pub anchor_y:         f32,
    /// Width of the output surface.
    pub viewport_width:   f32,
    /// Height of the output surface.
    pub viewport_height:  f32,
    /// Preferred menu width, already resolved from its content or size class.
    pub menu_width:       f32,
    /// Height the menu content needs.
    pub content_height:   f32,
    /// Height available to the menu before it must scroll.
    pub available_height: f32,
    /// Themed text size, the unit every em length is stated in.
    pub font_size:        f32,
    /// How far the open animation has travelled, zero closed and one open.
    pub progress:         f32,
    /// Gap between the bar edge and the drawer. Zero for a joined drawer.
    pub edge_gap:         f32
}

/// Resolved placement of a drawer menu inside its full-screen surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawerPlacement {
    /// Offset from the attached bar edge, including slide.
    pub main_offset:     f32,
    /// Offset along the bar edge, clamped to keep the menu on screen.
    pub cross_offset:    f32,
    /// Width the menu box may take.
    pub width:           f32,
    /// Height the menu box may take before scrolling.
    pub menu_height:     f32,
    /// Distance the drawer still has to travel to be fully open.
    pub slide_remaining: f32
}

/// Solves the placement of a drawer menu.
///
/// The drawer is attached to `bar_position` with `edge_gap` (zero for a joined
/// drawer). It slides out from that edge by `SLIDE_EM * (1 - progress)`. The
/// cross-axis offset is clamped so the menu stays within the screen margins.
#[must_use]
pub fn solve(input: PlacementInput) -> DrawerPlacement {
    let progress = input.progress.clamp(0.0, 1.0);
    let slide_remaining = SLIDE_EM * input.font_size * (1.0 - progress);
    let main_offset = input.edge_gap.max(0.0) + slide_remaining;

    let width = resolve_width(input.menu_width, input.viewport_width, input.font_size);
    let menu_height = resolve_height(
        input.content_height,
        input.available_height,
        input.viewport_height
    );
    let cross_offset = resolve_cross_offset(&input, width, menu_height);

    DrawerPlacement {
        main_offset,
        cross_offset,
        width,
        menu_height,
        slide_remaining
    }
}

fn resolve_width(preferred: f32, viewport_width: f32, font_size: f32) -> f32 {
    let floor = MIN_WIDTH_EM * font_size;
    let ceiling = (viewport_width * MAX_WIDTH_SHARE).max(1.0);

    preferred.clamp(floor.min(ceiling), ceiling)
}

fn resolve_height(content_height: f32, available_height: f32, viewport_height: f32) -> f32 {
    let max_height = if available_height > 0.0 {
        available_height
    } else {
        (viewport_height * MAX_HEIGHT_SHARE).max(1.0)
    };

    content_height.clamp(0.0, max_height.max(0.0))
}

fn resolve_cross_offset(input: &PlacementInput, menu_width: f32, menu_height: f32) -> f32 {
    let margin = EDGE_MARGIN_EM * input.font_size;

    match input.bar_position {
        Position::Top | Position::Bottom => {
            clamp_offset(input.anchor_x, menu_width, input.viewport_width, margin)
        }
        Position::Left | Position::Right => {
            clamp_offset(input.anchor_y, menu_height, input.viewport_height, margin)
        }
    }
}

fn clamp_offset(anchor: f32, size: f32, viewport: f32, margin: f32) -> f32 {
    let max_offset = (viewport - size - margin).max(margin);
    (anchor - size / 2.0).clamp(margin, max_offset)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn input() -> PlacementInput {
        PlacementInput {
            bar_position:     Position::Top,
            anchor_x:         500.0,
            anchor_y:         20.0,
            viewport_width:   1920.0,
            viewport_height:  1080.0,
            menu_width:       300.0,
            content_height:   200.0,
            available_height: 900.0,
            font_size:        16.0,
            progress:         1.0,
            edge_gap:         0.0
        }
    }

    #[test]
    fn joined_drawer_has_zero_edge_gap_at_full_progress() {
        let placement = solve(input());

        assert_eq!(placement.main_offset, 0.0);
        assert_eq!(placement.slide_remaining, 0.0);
    }

    #[test]
    fn opening_drawer_slides_from_bar_edge() {
        let placement = solve(PlacementInput {
            progress: 0.0,
            ..input()
        });

        let expected = SLIDE_EM * 16.0;

        assert!((placement.main_offset - expected).abs() < f32::EPSILON);
        assert!(placement.slide_remaining > 0.0);
    }

    #[test]
    fn cross_offset_is_clamped_to_screen_margin() {
        let placement = solve(PlacementInput {
            anchor_x: 10.0,
            ..input()
        });
        let margin = EDGE_MARGIN_EM * 16.0;

        assert!(placement.cross_offset >= margin);
    }

    #[test]
    fn cross_offset_is_clamped_to_right_edge() {
        let placement = solve(PlacementInput {
            anchor_x: 1900.0,
            ..input()
        });
        let margin = EDGE_MARGIN_EM * 16.0;

        assert!(placement.cross_offset + placement.width <= 1920.0 - margin + 0.01);
    }

    #[test]
    fn width_is_clamped_to_viewport_share() {
        let placement = solve(PlacementInput {
            menu_width: 5000.0,
            ..input()
        });

        let ceiling = 1920.0 * MAX_WIDTH_SHARE;

        assert!(placement.width <= ceiling + 0.01);
    }

    #[test]
    fn height_is_clamped_to_available_height() {
        let placement = solve(PlacementInput {
            content_height: 2000.0,
            available_height: 500.0,
            ..input()
        });

        assert!((placement.menu_height - 500.0).abs() < f32::EPSILON);
    }

    #[test]
    fn left_bar_uses_vertical_cross_axis() {
        let placement = solve(PlacementInput {
            bar_position: Position::Left,
            anchor_y: 100.0,
            ..input()
        });

        assert!(placement.cross_offset >= 0.0);
        assert_eq!(placement.main_offset, 0.0);
    }

    #[test]
    fn bottom_bar_uses_horizontal_cross_axis() {
        let placement = solve(PlacementInput {
            bar_position: Position::Bottom,
            ..input()
        });

        assert_eq!(placement.main_offset, 0.0);
        assert!(placement.cross_offset >= 0.0);
    }

    #[test]
    fn right_bar_slides_from_right_edge() {
        let placement = solve(PlacementInput {
            bar_position: Position::Right,
            progress: 0.5,
            ..input()
        });

        let expected = SLIDE_EM * 16.0 * 0.5;
        assert!((placement.main_offset - expected).abs() < 0.001);
    }
}
