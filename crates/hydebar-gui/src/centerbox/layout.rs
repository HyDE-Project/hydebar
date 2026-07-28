//! Placement of the three sections a [`Centerbox`] distributes.

use iced::{
    Alignment, Element, Length, Point, Size,
    advanced::{
        layout::{Limits, Node},
        widget::Tree
    }
};

use super::builder::Centerbox;

/// Lays the three sections out over the content box of the [`Centerbox`].
///
/// The edges are measured first so the centre section only ever claims what is
/// left of the row, and the right section is anchored at the end of the content
/// box instead of after its own siblings.
pub(super) fn layout<'a, Message, Theme, Renderer>(
    centerbox: &mut Centerbox<'a, Message, Theme, Renderer>,
    tree: &mut Tree,
    renderer: &Renderer,
    limits: &Limits
) -> Node
where
    Renderer: iced::advanced::Renderer
{
    let limits = limits
        .width(centerbox.width)
        .height(centerbox.height)
        .shrink(centerbox.padding);

    let total_spacing = centerbox.spacing * 3_i32.saturating_sub(1) as f32;
    let max_cross = limits.max().height;

    let mut cross = match centerbox.height {
        Length::Shrink => 0.0,
        _ => max_cross
    };

    let available = limits.max().width - total_spacing;

    let mut nodes = [Node::default(), Node::default(), Node::default()];

    let mut remaining = match centerbox.width {
        Length::Shrink => 0.0,
        _ => available.max(0.0)
    };

    let mut calculate_edge_layout =
        |i: usize, (child, tree): (&mut Element<'a, Message, Theme, Renderer>, &mut Tree)| {
            let fill_cross_factor = {
                let size = child.as_widget_mut().size();

                size.height.fill_factor()
            };

            let (max_width, max_height) = (
                remaining,
                if fill_cross_factor != 0 {
                    cross
                } else {
                    max_cross
                }
            );

            let child_limits = Limits::new(Size::ZERO, Size::new(max_width, max_height));

            let layout = child.as_widget_mut().layout(tree, renderer, &child_limits);
            let size = layout.size();

            remaining -= size.width;
            cross = cross.max(size.height);

            nodes[i] = layout;
        };

    calculate_edge_layout(0, (&mut centerbox.children[0], &mut tree.children[0]));
    calculate_edge_layout(2, (&mut centerbox.children[2], &mut tree.children[2]));
    calculate_edge_layout(1, (&mut centerbox.children[1], &mut tree.children[1]));

    let content_start = centerbox.padding.left;
    let content_end = content_start + limits.max().width;

    nodes[0].move_to_mut(Point::new(content_start, centerbox.padding.top));
    nodes[0].align_mut(
        Alignment::Start,
        centerbox.align_items,
        Size::new(0.0, cross)
    );
    nodes[2].move_to_mut(Point::new(content_end, centerbox.padding.top));
    nodes[2].align_mut(Alignment::End, centerbox.align_items, Size::new(0.0, cross));

    let half_available = available / 2.0;
    let half_center_width = nodes[1].size().width / 2.0;

    if half_available - nodes[0].size().width < half_center_width
        || half_available - nodes[2].size().width < half_center_width
    {
        nodes[1].move_to_mut(Point::new(
            content_start
                + centerbox.spacing
                + nodes[0].size().width
                + (available - nodes[0].size().width - nodes[2].size().width) / 2.0,
            centerbox.padding.top
        ));
    } else {
        nodes[1].move_to_mut(Point::new(
            (content_start + content_end) / 2.0,
            centerbox.padding.top
        ));
    }
    nodes[1].align_mut(
        Alignment::Center,
        centerbox.align_items,
        Size::new(0.0, cross)
    );

    let main =
        nodes[0].size().width + nodes[1].size().width + nodes[2].size().width + total_spacing;

    let (intrinsic_width, intrinsic_height) = (main, cross);
    let size = limits.resolve(
        centerbox.width,
        centerbox.height,
        Size::new(intrinsic_width, intrinsic_height)
    );

    Node::with_children(size.expand(centerbox.padding), nodes.into())
}
