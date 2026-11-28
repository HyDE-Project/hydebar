//! Placement of a menu box on the full screen menu surface.

use iced::{
    Element, Length, Padding, SurfaceId as Id,
    alignment::{Horizontal, Vertical},
    widget::{Space, canvas, column, container, mouse_area, row, scrollable}
};

use super::{
    flare::{ConcaveFlare, FlareCorner},
    placement::{PlacementInput, solve},
    size::MenuSize
};
use crate::{
    config::{AppearanceStyle, Position},
    position_button::ButtonUIRef,
    style::{menu_backdrop_style, menu_container_style}
};

/// Inner padding of a menu box, in multiples of the themed text size.
pub const PADDING_EM: f32 = 1.6;

/// Everything the placement needs to know about the theme in force.
#[derive(Debug, Clone, Copy)]
pub struct MenuLayout {
    /// Themed text size, the unit every length here is stated in.
    pub font_size:        f32,
    /// Screen edge the bar is docked to.
    pub bar_position:     Position,
    /// Style the bar is drawn in.
    pub style:            AppearanceStyle,
    /// Opacity of the menu box.
    pub opacity:          f32,
    /// Corner radius of the menu box, matching the bar pills.
    pub radius:           f32,
    /// Opacity of the dimming behind the menu.
    pub menu_backdrop:    f32,
    /// Border and shadow the window adopts from the compositor.
    pub finish:           crate::style::IslandFinish,
    /// Height the content needs, when the caller can measure it.
    ///
    /// Given one the box is capped to the share of the screen a menu may cover
    /// and its content scrolls, so a long page is reachable instead of being
    /// cut off by the edge of the screen.
    pub content_height:   Option<f32>,
    /// Height the surface can give the box before it must scroll.
    ///
    /// Learned from the screen the bar stands on, never from the viewport of
    /// the button that opened the menu — that viewport is the bar strip, and
    /// capping to it once squashed every window on the desktop.
    pub available_height: Option<f32>,

    /// How far the open animation has travelled, zero closed and one open.
    ///
    /// The box slides in from the bar edge by the remainder of this while it
    /// fades, so the window reads as growing out of the module that owns it
    /// rather than materialising in place.
    pub progress: f32
}

/// Height the box takes while the open animation is still travelling.
///
/// The box grows out of the bar edge instead of materialising at full size:
/// `progress` below one pins it to the travelled share of its resting height,
/// clipped so the content is revealed rather than squeezed. At one the caller
/// keeps the resting height, so the end of the animation lands exactly where
/// the box would sit without one. `cap` is the screen room that caps a tall
/// box into scrolling; a box that fits grows to its content instead.
#[must_use]
pub fn grown_height(content: f32, padding: f32, cap: Option<f32>, progress: f32) -> f32 {
    let full = padding.mul_add(2.0, content);
    let rest = cap.filter(|room| full > *room).unwrap_or(full);

    rest * progress.clamp(0.0, 1.0)
}

fn add_concave_flares<'a, Message: 'a>(
    menu_box: Element<'a, Message>,
    layout: &MenuLayout,
    width: f32
) -> Element<'a, Message> {
    let r = layout.radius.max(0.0);
    if r <= 0.0 {
        return menu_box;
    }

    let opacity = layout.opacity;
    let flare = |corner| {
        canvas(ConcaveFlare {
            radius: r,
            corner,
            opacity
        })
        .width(Length::Fixed(r))
        .height(Length::Fixed(r))
    };

    match layout.bar_position {
        Position::Top | Position::Bottom => {
            let (left_corner, right_corner) = match layout.bar_position {
                Position::Top => (FlareCorner::BottomRight, FlareCorner::BottomLeft),
                Position::Bottom => (FlareCorner::TopRight, FlareCorner::TopLeft),
                _ => unreachable!()
            };

            let left_col = if layout.bar_position == Position::Top {
                column![flare(left_corner), Space::new().height(Length::Fill),]
                    .width(Length::Fixed(r))
            } else {
                column![Space::new().height(Length::Fill), flare(left_corner),]
                    .width(Length::Fixed(r))
            };

            let right_col = if layout.bar_position == Position::Top {
                column![flare(right_corner), Space::new().height(Length::Fill),]
                    .width(Length::Fixed(r))
            } else {
                column![Space::new().height(Length::Fill), flare(right_corner),]
                    .width(Length::Fixed(r))
            };

            row![left_col, menu_box, right_col]
                .width(Length::Fixed(r.mul_add(2.0, width)))
                .height(Length::Shrink)
                .into()
        }
        Position::Left | Position::Right => {
            let (top_corner, bottom_corner) = match layout.bar_position {
                Position::Left => (FlareCorner::BottomRight, FlareCorner::TopRight),
                Position::Right => (FlareCorner::BottomLeft, FlareCorner::TopLeft),
                _ => unreachable!()
            };

            let make_row = |corner| {
                if layout.bar_position == Position::Left {
                    row![Space::new().width(Length::Fill), flare(corner),].height(Length::Fixed(r))
                } else {
                    row![flare(corner), Space::new().width(Length::Fill),].height(Length::Fixed(r))
                }
            };

            let flare_col = column![
                make_row(top_corner),
                Space::new().height(Length::Fill),
                make_row(bottom_corner)
            ]
            .width(Length::Fixed(r))
            .height(Length::Fill);

            match layout.bar_position {
                Position::Left => row![flare_col, menu_box].height(Length::Shrink).into(),
                Position::Right => row![menu_box, flare_col].height(Length::Shrink).into(),
                _ => unreachable!()
            }
        }
    }
}

/// Computes the outer padding that places the menu against the bar edge.
fn menu_padding(
    layout: &MenuLayout,
    inner_padding: f32,
    growing: Option<f32>,
    overflows: bool,
    width: f32,
    viewport_width: f32,
    button_ui_ref: &ButtonUIRef
) -> Padding {
    let menu_height = growing.unwrap_or_else(|| {
        if overflows {
            layout
                .available_height
                .unwrap_or_else(|| MenuSize::max_height(button_ui_ref.viewport.1))
        } else {
            inner_padding.mul_add(2.0, layout.content_height.unwrap_or(0.0))
        }
    });

    let placement = solve(PlacementInput {
        bar_position: layout.bar_position,
        anchor_x: button_ui_ref.position.x,
        anchor_y: button_ui_ref.position.y,
        viewport_width,
        viewport_height: button_ui_ref.viewport.1,
        menu_width: width,
        content_height: menu_height,
        available_height: menu_height,
        font_size: layout.font_size,
        progress: layout.progress,
        edge_gap: 0.0
    });

    match layout.bar_position {
        Position::Top => Padding::new(0.)
            .top(placement.main_offset)
            .left(placement.cross_offset),
        Position::Bottom => Padding::new(0.)
            .bottom(placement.main_offset)
            .left(placement.cross_offset),
        Position::Left => Padding::new(0.)
            .left(placement.main_offset)
            .top(placement.cross_offset),
        Position::Right => Padding::new(0.)
            .right(placement.main_offset)
            .top(placement.cross_offset)
    }
}

/// Wraps `content` into a menu box anchored under `button_ui_ref`.
///
/// The box takes the width `menu_size` asks for on the output it lands on,
/// is nudged back inside the screen when its button sits near an
/// edge. Content measured taller than the share of the screen a menu may
/// cover scrolls inside the box instead of running off the edge.
pub fn menu_wrapper<Message: Clone + 'static>(
    _id: Id,
    content: Element<'_, Message>,
    menu_size: MenuSize,
    button_ui_ref: ButtonUIRef,
    layout: MenuLayout,
    none_message: Message,
    close_menu_message: Message
) -> Element<'_, Message> {
    let (viewport_width, _viewport_height) = button_ui_ref.viewport;
    let width = menu_size.width(layout.font_size, viewport_width);
    let padding = PADDING_EM * layout.font_size;

    let overflows = match (layout.content_height, layout.available_height) {
        (Some(content), Some(room)) => padding.mul_add(2.0, content) > room,
        _ => false
    };

    let growing = match layout.content_height {
        Some(content) if layout.progress < 1.0 => Some(grown_height(
            content,
            padding,
            layout.available_height,
            layout.progress
        )),
        _ => None
    };

    let body: Element<'_, Message> = if overflows {
        scrollable(content)
            .width(Length::Fill)
            .spacing(layout.font_size * 0.4)
            .into()
    } else {
        content
    };

    let mut menu_box = container(body)
        .height(Length::Shrink)
        .width(Length::Fill)
        .max_width(width)
        .padding(padding);

    if overflows && let Some(room) = layout.available_height {
        menu_box = menu_box.height(Length::Fixed(room));
    }

    if let Some(height) = growing {
        menu_box = menu_box.height(Length::Fixed(height)).clip(true);
    }

    let menu_box = menu_box.style(menu_container_style(
        layout.opacity,
        layout.radius,
        layout.finish,
        Some(layout.bar_position)
    ));

    let menu_box = add_concave_flares(menu_box.into(), &layout, width);

    mouse_area(
        container(
            mouse_area(menu_box)
                .on_press(none_message.clone())
                .on_right_press(none_message)
        )
        .align_x(match layout.bar_position {
            Position::Top | Position::Bottom | Position::Left => Horizontal::Left,
            Position::Right => Horizontal::Right
        })
        .align_y(match layout.bar_position {
            Position::Top | Position::Left | Position::Right => Vertical::Top,
            Position::Bottom => Vertical::Bottom
        })
        .padding({
            let mut pad = menu_padding(
                &layout,
                padding,
                growing,
                overflows,
                width,
                viewport_width,
                &button_ui_ref
            );
            let r = layout.radius.max(0.0);
            if r > 0.0 {
                match layout.bar_position {
                    Position::Top | Position::Bottom | Position::Left => {
                        pad.left = (pad.left - r).max(0.0);
                    }
                    Position::Right => {
                        pad.right = (pad.right - r).max(0.0);
                    }
                }
            }
            pad
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .style(menu_backdrop_style(
            layout.menu_backdrop * layout.progress.clamp(0.0, 1.0)
        ))
    )
    .on_press(close_menu_message)
    .into()
}
