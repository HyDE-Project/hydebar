//! Seating the children and keeping the book of settled seats current.

use iced::{
    Size,
    core::{layout, widget::Tree}
};

use super::{builder::SlidingRow, state::State};

/// Lays the children out left to right and records their settled seats.
///
/// While the row rests, the seats it assigns become the book the next
/// slide departs from; while a slide travels, the departure book is
/// frozen so every frame of the journey measures against the same start.
pub(super) fn layout<Message, Theme, Renderer>(
    row: &mut SlidingRow<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    renderer: &Renderer,
    limits: &layout::Limits
) -> layout::Node
where
    Renderer: iced_core::Renderer
{
    let loose = limits.loose();
    let mut nodes = Vec::with_capacity(row.children.len());
    let mut x = 0.0f32;
    let mut height = 0.0f32;

    for (child, sub_tree) in row.children.iter_mut().zip(tree.children.iter_mut()) {
        let node = child.as_widget_mut().layout(sub_tree, renderer, &loose);
        height = height.max(node.size().height);
        nodes.push((x, node));
        x += nodes.last().map_or(0.0, |(_, node)| node.size().width) + row.spacing;
    }

    let width = (x - row.spacing).max(0.0);

    let placed: Vec<layout::Node> = nodes
        .into_iter()
        .map(|(x, node)| {
            let dy = (height - node.size().height) / 2.0;
            node.move_to(iced::Point::new(x, dy))
        })
        .collect();

    let state = tree.state.downcast_mut::<State>();

    if row.progress >= 1.0 {
        state.settled = row
            .keys
            .iter()
            .zip(placed.iter())
            .map(|(key, node)| (*key, node.bounds().x))
            .collect();
        state.from.clear();
    } else if state.from.is_empty() {
        state.from = state.settled.clone();
    }

    layout::Node::with_children(Size::new(width, height), placed)
}
