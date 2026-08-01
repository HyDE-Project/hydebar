//! Painting of the sections a [`Centerbox`] holds.

use iced::{
    Rectangle,
    advanced::{layout::Layout, mouse, renderer, widget::Tree}
};

use super::builder::Centerbox;

/// Paints every section clipped to the box the [`Centerbox`] occupies.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors the parameter list of the iced widget draw contract"
)]
pub(super) fn draw<Message, Theme, Renderer>(
    centerbox: &Centerbox<'_, Message, Theme, Renderer>,
    tree: &Tree,
    renderer: &mut Renderer,
    theme: &Theme,
    style: &renderer::Style,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    viewport: &Rectangle
) where
    Renderer: iced::advanced::Renderer
{
    if let Some(viewport) = layout.bounds().intersection(viewport) {
        for ((child, state), layout) in centerbox
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(state, renderer, theme, style, layout, cursor, &viewport);
        }
    }
}
