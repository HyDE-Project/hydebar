//! Delegation of the interaction of a [`Centerbox`] to its sections.

use iced::{
    Rectangle, Vector,
    advanced::{
        Clipboard, Shell,
        layout::Layout,
        mouse, overlay,
        widget::{Operation, Tree}
    }
};
use iced_core::Event;

use super::builder::Centerbox;

/// Runs an operation over the [`Centerbox`] and every section it holds.
pub(super) fn operate<Message, Theme, Renderer>(
    centerbox: &mut Centerbox<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &Renderer,
    operation: &mut dyn Operation
) where
    Renderer: iced::advanced::Renderer
{
    operation.container(None, layout.bounds());
    centerbox
        .children
        .iter_mut()
        .zip(&mut tree.children)
        .zip(layout.children())
        .for_each(|((child, state), layout)| {
            child
                .as_widget_mut()
                .operate(state, layout, renderer, operation);
        });
}

/// Feeds an event to every section of the [`Centerbox`].
pub(super) fn update<Message, Theme, Renderer>(
    centerbox: &mut Centerbox<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    event: &Event,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    renderer: &Renderer,
    clipboard: &mut dyn Clipboard,
    shell: &mut Shell<'_, Message>,
    viewport: &Rectangle
) where
    Renderer: iced::advanced::Renderer
{
    centerbox
        .children
        .iter_mut()
        .zip(&mut tree.children)
        .zip(layout.children())
        .for_each(|((child, state), layout)| {
            child.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport
            );
        });
}

/// Reports the strongest interaction any section of the [`Centerbox`] asks for.
pub(super) fn mouse_interaction<Message, Theme, Renderer>(
    centerbox: &Centerbox<'_, Message, Theme, Renderer>,
    tree: &Tree,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    viewport: &Rectangle,
    renderer: &Renderer
) -> mouse::Interaction
where
    Renderer: iced::advanced::Renderer
{
    centerbox
        .children
        .iter()
        .zip(&tree.children)
        .zip(layout.children())
        .map(|((child, state), layout)| {
            child
                .as_widget()
                .mouse_interaction(state, layout, cursor, viewport, renderer)
        })
        .max()
        .unwrap_or_default()
}

/// Collects the overlay published by the sections of the [`Centerbox`].
pub(super) fn overlay<'a, 'b, Message, Theme, Renderer>(
    centerbox: &'b mut Centerbox<'a, Message, Theme, Renderer>,
    tree: &'b mut Tree,
    layout: Layout<'b>,
    renderer: &Renderer,
    viewport: &Rectangle,
    translation: Vector
) -> Option<iced_core::overlay::Element<'b, Message, Theme, Renderer>>
where
    Renderer: iced::advanced::Renderer
{
    let children: &'b mut [iced_core::Element<'a, Message, Theme, Renderer>] =
        &mut centerbox.children;

    overlay::from_children(children, tree, layout, renderer, viewport, translation)
}
