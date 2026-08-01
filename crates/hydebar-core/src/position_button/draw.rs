use iced::{
    Background, Color, Rectangle,
    core::{Layout, mouse, renderer, widget::Tree},
    widget::button::Catalog
};

use super::{
    builder::PositionButton,
    state::{State, resolve_status}
};

/// Paints the background of the button and then its content.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors the upstream widget draw signature"
)]
pub(super) fn draw<Message, Theme, Renderer>(
    button: &PositionButton<'_, Message, Theme, Renderer>,
    tree: &Tree,
    renderer: &mut Renderer,
    theme: &Theme,
    _renderer_style: &renderer::Style,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    viewport: &Rectangle
) where
    Renderer: iced_core::Renderer,
    Theme: Catalog
{
    let bounds = layout.bounds();
    let content_layout = layout.children().next().unwrap();
    let state = tree.state.downcast_ref::<State>();

    let status = resolve_status(
        button.is_pressable(),
        cursor.is_over(bounds),
        state.is_pressed()
    );

    let style = theme.style(&button.class, status);

    if style.background.is_some() || style.border.width > 0.0 || style.shadow.color.a > 0.0 {
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: style.border,
                shadow: style.shadow,
                snap: false
            },
            style
                .background
                .unwrap_or(Background::Color(Color::TRANSPARENT))
        );
    }

    let viewport = if button.clip {
        bounds.intersection(viewport).unwrap_or(*viewport)
    } else {
        *viewport
    };

    button.content.as_widget().draw(
        &tree.children[0],
        renderer,
        theme,
        &renderer::Style {
            text_color: style.text_color
        },
        content_layout,
        cursor,
        &viewport
    );
}

#[cfg(test)]
mod tests {
    use iced::{
        Padding, Point, Size, Theme,
        core::{Widget, layout::Limits}
    };

    use super::*;
    use crate::position_button::position_button;

    const BOUNDS: Rectangle = Rectangle {
        x:      0.0,
        y:      0.0,
        width:  36.0,
        height: 26.0
    };

    fn cursor_at(x: f32) -> mouse::Cursor {
        mouse::Cursor::Available(Point::new(x, 10.0))
    }

    /// Renderer keeping every quad the button paints.
    #[derive(Default)]
    struct QuadRecorder {
        quads: Vec<renderer::Quad>
    }

    impl iced_core::Renderer for QuadRecorder {
        fn start_layer(&mut self, _bounds: Rectangle) {}

        fn end_layer(&mut self) {}

        fn start_transformation(&mut self, _transformation: iced::Transformation) {}

        fn end_transformation(&mut self) {}

        fn reset(&mut self, _new_bounds: Rectangle) {}

        fn fill_quad(&mut self, quad: renderer::Quad, _background: impl Into<Background>) {
            self.quads.push(quad);
        }

        fn allocate_image(
            &mut self,
            _handle: &iced_core::image::Handle,
            _callback: impl FnOnce(Result<iced_core::image::Allocation, iced_core::image::Error>)
            + Send
            + 'static
        ) {
            unreachable!("a button never draws an image")
        }
    }

    const PILL_RADIUS: f32 = 4.0;

    /// Paints a hovered module button and reports the quads it filled together
    /// with the bounds it was laid out with.
    fn painted_hover(padding: Padding) -> (Vec<renderer::Quad>, Rectangle) {
        let style = || {
            crate::style::module_button_style(
                crate::config::AppearanceStyle::Islands,
                1.0,
                PILL_RADIUS,
                false,
                false,
                1.0,
                crate::style::IslandFinish::bare()
            )
        };

        let content = || iced::widget::Space::new().width(16.0).height(16.0);

        let mut tree = Tree::new(iced_core::Element::<(), Theme, QuadRecorder>::new(
            position_button(content()).padding(padding).style(style())
        ));

        let mut button: PositionButton<'_, (), Theme, QuadRecorder> = position_button(content())
            .padding(padding)
            .style(style())
            .on_press(());

        let node = button.layout(
            &mut tree,
            &QuadRecorder::default(),
            &Limits::new(Size::ZERO, Size::new(200.0, 26.0))
        );

        let mut renderer = QuadRecorder::default();

        button.draw(
            &tree,
            &mut renderer,
            &Theme::Dark,
            &renderer::Style::default(),
            Layout::new(&node),
            cursor_at(10.0),
            &BOUNDS
        );

        (renderer.quads, Layout::new(&node).bounds())
    }

    #[test]
    fn the_hover_background_covers_the_button_bounds_and_nothing_more() {
        let padding = Padding {
            top:    2.0,
            bottom: 2.0,
            right:  3.0,
            left:   3.0
        };

        let (quads, bounds) = painted_hover(padding);

        assert_eq!(quads.len(), 1, "the hover paints a single background quad");
        assert_eq!(
            quads[0].bounds, bounds,
            "the hover background has to match the box the button was laid out \
             with, so it never spills over the padding of the island hosting it"
        );
        assert_eq!(
            bounds.size(),
            Size::new(
                16.0 + padding.left + padding.right,
                16.0 + padding.top + padding.bottom
            ),
            "the button box is its content grown by the module padding"
        );
        assert_eq!(
            quads[0].border.radius,
            PILL_RADIUS.into(),
            "the hover pill is rounded like the island it is drawn in"
        );
    }
}
