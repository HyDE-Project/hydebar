//! Delivering events and cursor questions to the children, with the
//! cursor shifted so a gliding child is hit where it is drawn.

use iced::{
    Rectangle, Vector,
    core::{Clipboard, Layout, Shell, event::Event, mouse, widget::Tree}
};

use super::{builder::SlidingRow, state::State};

/// Routes `event` to every child, shifting the cursor by each offset.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors the upstream widget update signature"
)]
pub(super) fn update<Message, Theme, Renderer>(
    row: &mut SlidingRow<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    event: &Event,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    renderer: &Renderer,
    clipboard: &mut dyn Clipboard,
    shell: &mut Shell<'_, Message>,
    viewport: &Rectangle
) where
    Renderer: iced_core::Renderer
{
    let origin = layout.position();
    let offsets: Vec<f32> = {
        let state = tree.state.downcast_ref::<State>();

        layout
            .children()
            .enumerate()
            .map(|(index, child_layout)| {
                row.offset(state, index, child_layout.bounds().x - origin.x)
            })
            .collect()
    };

    for (index, ((child, sub_tree), child_layout)) in row
        .children
        .iter_mut()
        .zip(tree.children.iter_mut())
        .zip(layout.children())
        .enumerate()
    {
        let shifted = match cursor {
            mouse::Cursor::Available(point) => {
                mouse::Cursor::Available(point - Vector::new(offsets[index], 0.0))
            }
            other => other
        };

        child.as_widget_mut().update(
            sub_tree,
            event,
            child_layout,
            shifted,
            renderer,
            clipboard,
            shell,
            viewport
        );
    }
}

/// Asks every child what the cursor means to it and keeps the strongest
/// answer.
pub(super) fn mouse_interaction<Message, Theme, Renderer>(
    row: &SlidingRow<'_, Message, Theme, Renderer>,
    tree: &Tree,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    viewport: &Rectangle,
    renderer: &Renderer
) -> mouse::Interaction
where
    Renderer: iced_core::Renderer
{
    let origin = layout.position();
    let state = tree.state.downcast_ref::<State>();

    row.children
        .iter()
        .zip(tree.children.iter())
        .zip(layout.children())
        .enumerate()
        .map(|(index, ((child, sub_tree), child_layout))| {
            let offset = row.offset(state, index, child_layout.bounds().x - origin.x);
            let shifted = match cursor {
                mouse::Cursor::Available(point) => {
                    mouse::Cursor::Available(point - Vector::new(offset, 0.0))
                }
                other => other
            };

            child.as_widget().mouse_interaction(
                sub_tree,
                child_layout,
                shifted,
                viewport,
                renderer
            )
        })
        .max()
        .unwrap_or_default()
}
