//! Placement of a menu box on the full screen menu surface.

use iced::{
    Element, Length, Padding, SurfaceId as Id,
    alignment::{Horizontal, Vertical},
    widget::{container, mouse_area, scrollable}
};

use super::size::MenuSize;
use crate::{
    config::{AppearanceStyle, Position},
    position_button::ButtonUIRef,
    style::{menu_backdrop_style, menu_container_style}
};

/// Distance the box travels while it fades in, in em of the themed size.
///
/// Small on purpose: the window should read as growing out of the bar, not
/// as flying across the screen.
const SLIDE_EM: f32 = 0.8;

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
        (Some(content), Some(room)) => content + padding * 2.0 > room,
        _ => false
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
        .padding(padding)
        .style(menu_container_style(layout.opacity, layout.radius, layout.finish));

    if overflows && let Some(room) = layout.available_height {
        menu_box = menu_box.height(Length::Fixed(room));
    }

    mouse_area(
        container(
            mouse_area(menu_box)
                .on_press(none_message.clone())
                .on_right_press(none_message)
        )
        .align_y(match layout.bar_position {
            Position::Top => Vertical::Top,
            Position::Bottom => Vertical::Bottom
        })
        .align_x(Horizontal::Left)
        .padding({
            let edge_gap = match layout.style {
                AppearanceStyle::Solid | AppearanceStyle::Gradient => 2.0,
                AppearanceStyle::Islands => 0.0
            };

            let slide = SLIDE_EM * layout.font_size * (1.0 - layout.progress.clamp(0.0, 1.0));

            Padding::new(0.)
                .top(if layout.bar_position == Position::Top {
                    edge_gap + slide
                } else {
                    0.0
                })
                .bottom(if layout.bar_position == Position::Bottom {
                    edge_gap + slide
                } else {
                    0.0
                })
                .left(MenuSize::left_offset(
                    width,
                    button_ui_ref.position.x,
                    viewport_width,
                    layout.font_size
                ))
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
