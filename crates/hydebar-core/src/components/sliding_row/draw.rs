//! Drawing the children where the slide currently holds them.

use iced::{
    Rectangle, Vector,
    core::{Layout, mouse, renderer, widget::Tree}
};

use super::{builder::SlidingRow, state::State};

/// Draws every child, translated by its offset while a slide travels.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors the upstream widget draw signature"
)]
pub(super) fn draw<Message, Theme, Renderer>(
    row: &SlidingRow<'_, Message, Theme, Renderer>,
    tree: &Tree,
    renderer: &mut Renderer,
    theme: &Theme,
    style: &renderer::Style,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    viewport: &Rectangle
) where
    Renderer: iced_core::Renderer
{
    let origin = layout.position();
    let state = tree.state.downcast_ref::<State>();

    for (index, ((child, sub_tree), child_layout)) in row
        .children
        .iter()
        .zip(tree.children.iter())
        .zip(layout.children())
        .enumerate()
    {
        let offset = row.offset(state, index, child_layout.bounds().x - origin.x);

        if offset.abs() < f32::EPSILON {
            child.as_widget().draw(
                sub_tree,
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport
            );
        } else {
            renderer.with_translation(Vector::new(offset, 0.0), |renderer| {
                child.as_widget().draw(
                    sub_tree,
                    renderer,
                    theme,
                    style,
                    child_layout,
                    cursor,
                    viewport
                );
            });
        }
    }
}
