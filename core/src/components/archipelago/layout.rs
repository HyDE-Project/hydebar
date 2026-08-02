//! Seating the modules, with room for a pill wherever an island opens or
//! closes.

use iced::{
    Size,
    core::{layout, widget::Tree}
};

use super::builder::Archipelago;

/// Lays the modules out left to right, padding the edges of every island.
pub(super) fn layout<Message, Theme, Renderer>(
    strip: &mut Archipelago<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    renderer: &Renderer,
    limits: &layout::Limits
) -> layout::Node
where
    Renderer: iced_core::Renderer
{
    let loose = limits.loose();
    let height = limits.max().height;
    let mut x = 0.0f32;
    let mut placed = Vec::with_capacity(strip.children.len());

    let count = strip.children.len();
    let islands = strip.islands.clone();
    let island_gap = strip.island_gap;
    let pad_x = strip.pad_x;

    for (index, (child, sub_tree)) in strip
        .children
        .iter_mut()
        .zip(tree.children.iter_mut())
        .enumerate()
    {
        let opens = index == 0 || islands[index] != islands[index - 1];
        let closes = index + 1 == count || islands[index + 1] != islands[index];

        if opens {
            if index > 0 {
                x += island_gap;
            }
            x += pad_x;
        }

        let node = child.as_widget_mut().layout(sub_tree, renderer, &loose);
        let dy = ((height - node.size().height) / 2.0).max(0.0);
        let width = node.size().width;

        placed.push(node.move_to(iced::Point::new(x, dy)));
        x += width;

        if closes {
            x += pad_x;
        }
    }

    layout::Node::with_children(Size::new(x, height), placed)
}
