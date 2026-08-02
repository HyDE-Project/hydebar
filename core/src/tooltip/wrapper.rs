//! Content of the tooltip surface: the box, its placement and its clamping.

use iced::{
    Element, Length, Padding,
    alignment::{Horizontal, Vertical},
    widget::{container, text}
};

use super::info::TooltipInfo;
use crate::{
    config::{Appearance, Position},
    position_button::ButtonUIRef,
    style::tooltip_container_style
};

/// Gap between the bar and the tooltip, in `em` of the themed font.
pub const TOOLTIP_GAP_EM: f32 = 0.4;

/// Padding inside the tooltip box, as `[vertical, horizontal]` `em`.
const TOOLTIP_PADDING_EM: [f32; 2] = [0.35, 0.7];

/// Advance width of a single tooltip glyph, in `em`.
///
/// Only used to guess how wide the box will be so it can be centred under the
/// module and kept away from the screen edges; the exact width is settled by
/// the layout engine afterwards. The guess is deliberately generous: a box
/// placed further left than it had to be is invisible, while one that leaves
/// its text too little room wraps a line that was meant to stay on one.
const GLYPH_ADVANCE_EM: f32 = 0.7;

/// Estimates how wide the tooltip box will be, in pixels.
///
/// Multi line tooltips are as wide as their longest line.
#[expect(
    clippy::cast_precision_loss,
    reason = "line lengths sit far below the f32 mantissa limit"
)]
fn estimated_width(text: &str, font_size: f32, horizontal_padding: f32) -> f32 {
    let glyphs = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or_default() as f32;

    horizontal_padding.mul_add(2., glyphs * font_size * GLYPH_ADVANCE_EM)
}

/// Places the left edge of the tooltip so it is centred under its module and
/// still fully on screen.
fn left_offset(anchor: &ButtonUIRef, width: f32, gap: f32) -> f32 {
    let rightmost = (anchor.viewport.0 - width - gap).max(gap);

    (anchor.position.x - width / 2.).clamp(gap, rightmost)
}

/// Builds the tooltip surface content showing `info`.
///
/// The surface starts where the bar ends, so the tooltip only has to add the
/// themed gap on the side the bar sits on.
#[must_use]
pub fn tooltip_wrapper<'a, Message: 'a>(
    info: &TooltipInfo,
    bar_position: Position,
    appearance: &Appearance
) -> Element<'a, Message> {
    let padding = [
        appearance.spacing(TOOLTIP_PADDING_EM[0]),
        appearance.spacing(TOOLTIP_PADDING_EM[1])
    ];
    let gap = appearance.spacing(TOOLTIP_GAP_EM);
    let width = estimated_width(&info.text, appearance.font_size_px(), padding[1]);

    container(
        container(text(info.text.clone()).size(appearance.font_size_px()))
            .padding(padding)
            .style(tooltip_container_style(
                appearance.menu.opacity,
                appearance.pill_radius()
            ))
    )
    .align_x(Horizontal::Left)
    .align_y(match bar_position {
        Position::Top => Vertical::Top,
        Position::Bottom => Vertical::Bottom
    })
    .padding(
        Padding::new(0.)
            .top(match bar_position {
                Position::Top => gap,
                Position::Bottom => 0.
            })
            .bottom(match bar_position {
                Position::Top => 0.,
                Position::Bottom => gap
            })
            .left(left_offset(&info.anchor, width, gap))
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]
    #![allow(clippy::suboptimal_flops)]

    use iced::Point;

    use super::*;

    fn anchor_at(x: f32) -> ButtonUIRef {
        ButtonUIRef {
            position: Point::new(x, 19.0),
            viewport: (1360.0, 38.0)
        }
    }

    #[test]
    fn the_width_guess_follows_the_longest_line() {
        let single = estimated_width("abcd", 10.0, 7.0);
        let multi = estimated_width("ab\nabcd\na", 10.0, 7.0);

        assert_eq!(single, multi);
        assert_eq!(single, 4.0 * 10.0 * GLYPH_ADVANCE_EM + 14.0);
    }

    #[test]
    fn a_tooltip_is_centred_under_its_module() {
        assert_eq!(left_offset(&anchor_at(680.0), 100.0, 4.0), 630.0);
    }

    #[test]
    fn a_tooltip_never_runs_off_the_left_edge() {
        assert_eq!(left_offset(&anchor_at(10.0), 100.0, 4.0), 4.0);
    }

    #[test]
    fn a_tooltip_never_runs_off_the_right_edge() {
        assert_eq!(left_offset(&anchor_at(1350.0), 100.0, 4.0), 1256.0);
    }

    #[test]
    fn a_tooltip_wider_than_the_screen_starts_at_the_left_edge() {
        assert_eq!(left_offset(&anchor_at(680.0), 4000.0, 4.0), 4.0);
    }
}
