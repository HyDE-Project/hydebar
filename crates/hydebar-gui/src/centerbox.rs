//! Distribute content horizontally.
use iced::{
    Alignment, Element, Event, Length, Padding, Pixels, Point, Rectangle, Size, Vector,
    advanced::{
        Clipboard, Shell, Widget,
        layout::{self, Layout, Limits, Node},
        mouse, overlay, renderer,
        widget::{Operation, Tree}
    }
};

/// A container that distributes its contents horizontally.
#[allow(missing_debug_implementations)]
pub struct Centerbox<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    spacing:     f32,
    padding:     Padding,
    width:       Length,
    height:      Length,
    align_items: Alignment,
    children:    [Element<'a, Message, Theme, Renderer>; 3]
}

impl<'a, Message, Theme, Renderer> Centerbox<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer
{
    /// Creates an empty [`Centerbox`].
    pub fn new(children: [Element<'a, Message, Theme, Renderer>; 3]) -> Self {
        Centerbox {
            spacing: 0.0,
            padding: Padding::ZERO,
            width: Length::Shrink,
            height: Length::Shrink,
            align_items: Alignment::Start,
            children
        }
    }

    /// Sets the horizontal spacing _between_ elements.
    ///
    /// Custom margins per element do not exist in iced. You should use this
    /// method instead! While less flexible, it helps you keep spacing between
    /// elements consistent.
    pub fn spacing(mut self, amount: impl Into<Pixels>) -> Self {
        self.spacing = amount.into().0;
        self
    }

    /// Sets the [`Padding`] of the [`Centerbox`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the width of the [`Centerbox`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Centerbox`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the vertical alignment of the contents of the [`Centerbox`] .
    pub fn align_items(mut self, align: Alignment) -> Self {
        self.align_items = align;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Centerbox<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer
{
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut self.children)
    }

    fn size(&self) -> Size<Length> {
        Size {
            width:  self.width,
            height: self.height
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits
    ) -> layout::Node {
        let limits = limits
            .width(self.width)
            .height(self.height)
            .shrink(self.padding);

        let total_spacing = self.spacing * 3_i32.saturating_sub(1) as f32;
        let max_cross = limits.max().height;

        let mut cross = match self.height {
            Length::Shrink => 0.0,
            _ => max_cross
        };

        let available = limits.max().width - total_spacing;

        let mut nodes = [Node::default(), Node::default(), Node::default()];

        let mut remaining = match self.width {
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

        calculate_edge_layout(0, (&mut self.children[0], &mut tree.children[0]));
        calculate_edge_layout(2, (&mut self.children[2], &mut tree.children[2]));
        calculate_edge_layout(1, (&mut self.children[1], &mut tree.children[1]));

        let content_start = self.padding.left;
        let content_end = content_start + limits.max().width;

        nodes[0].move_to_mut(Point::new(content_start, self.padding.top));
        nodes[0].align_mut(Alignment::Start, self.align_items, Size::new(0.0, cross));
        nodes[2].move_to_mut(Point::new(content_end, self.padding.top));
        nodes[2].align_mut(Alignment::End, self.align_items, Size::new(0.0, cross));

        let half_available = available / 2.0;
        let half_center_width = nodes[1].size().width / 2.0;

        if half_available - nodes[0].size().width < half_center_width
            || half_available - nodes[2].size().width < half_center_width
        {
            nodes[1].move_to_mut(Point::new(
                content_start
                    + self.spacing
                    + nodes[0].size().width
                    + (available - nodes[0].size().width - nodes[2].size().width) / 2.0,
                self.padding.top
            ));
        } else {
            nodes[1].move_to_mut(Point::new(
                (content_start + content_end) / 2.0,
                self.padding.top
            ));
        }
        nodes[1].align_mut(Alignment::Center, self.align_items, Size::new(0.0, cross));

        let main =
            nodes[0].size().width + nodes[1].size().width + nodes[2].size().width + total_spacing;

        let (intrinsic_width, intrinsic_height) = (main, cross);
        let size = limits.resolve(
            self.width,
            self.height,
            Size::new(intrinsic_width, intrinsic_height)
        );

        Node::with_children(size.expand(self.padding), nodes.into())
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation
    ) {
        operation.container(None, layout.bounds());
        self.children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .for_each(|((child, state), layout)| {
                child
                    .as_widget_mut()
                    .operate(state, layout, renderer, operation);
            });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle
    ) {
        self.children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .for_each(|((child, state), layout)| {
                child.as_widget_mut().update(
                    state, event, layout, cursor, renderer, clipboard, shell, viewport
                );
            });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer
    ) -> mouse::Interaction {
        self.children
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

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle
    ) {
        if let Some(viewport) = layout.bounds().intersection(viewport) {
            for ((child, state), layout) in self
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

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Centerbox<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a
{
    fn from(row: Centerbox<'a, Message, Theme, Renderer>) -> Self {
        Self::new(row)
    }
}

#[cfg(test)]
mod tests {
    use hydebar_core::position_button::position_button;
    use iced::widget::{Space, container, row};

    use super::*;

    type TestRenderer = ();
    type TestTheme = iced::Theme;

    const BAR_WIDTH: f32 = 1920.0;
    const BAR_HEIGHT: f32 = 34.0;
    const MODULE_PADDING: [f32; 2] = [2.0, 8.0];
    const MODULE_GAP: f32 = 4.0;
    const GROUP_GAP: f32 = 8.0;

    /// Builds a bar module shaped like the real ones: a fill-height button
    /// wrapping fixed-size content.
    fn module<'a>(width: f32) -> Element<'a, (), TestTheme, TestRenderer> {
        position_button(
            container(Space::new().width(width).height(16.0))
                .align_y(Alignment::Center)
                .height(Length::Fill)
        )
        .padding(MODULE_PADDING)
        .height(Length::Fill)
        .on_press(())
        .into()
    }

    /// Builds a bar section the way `modules_section` does.
    fn section<'a>(widths: &[f32]) -> Element<'a, (), TestTheme, TestRenderer> {
        let mut section = row!()
            .height(Length::Shrink)
            .align_y(Alignment::Center)
            .spacing(MODULE_GAP);

        for width in widths {
            section = section.push(module(*width));
        }

        section.into()
    }

    fn lay_out<'a>(
        children: [Element<'a, (), TestTheme, TestRenderer>; 3],
        padding: Padding,
        width: f32
    ) -> Node {
        let mut element: Element<'a, (), TestTheme, TestRenderer> = Centerbox::new(children)
            .spacing(GROUP_GAP)
            .width(Length::Fill)
            .height(BAR_HEIGHT)
            .align_items(Alignment::Center)
            .padding(padding)
            .into();

        let mut tree = Tree::new(&element);

        element.as_widget_mut().layout(
            &mut tree,
            &(),
            &Limits::new(Size::ZERO, Size::new(width, BAR_HEIGHT))
        )
    }

    /// Returns the index of the section whose bounds contain `x`.
    fn section_at(node: &Node, x: f32) -> Option<usize> {
        let layout = Layout::new(node);

        layout
            .children()
            .position(|section| section.bounds().contains(Point::new(x, BAR_HEIGHT / 2.0)))
    }

    /// Returns the `(section, module)` index pair whose bounds contain `x`.
    fn module_at(node: &Node, x: f32) -> Option<(usize, usize)> {
        let layout = Layout::new(node);
        let point = Point::new(x, BAR_HEIGHT / 2.0);

        layout.children().enumerate().find_map(|(section, layout)| {
            layout
                .children()
                .position(|module| module.bounds().contains(point))
                .map(|module| (section, module))
        })
    }

    #[test]
    fn sections_span_the_whole_content_box() {
        let padding = Padding::from([3.0, 10.0]);
        let node = lay_out(
            [
                section(&[40.0, 60.0]),
                section(&[80.0]),
                section(&[30.0, 50.0, 20.0])
            ],
            padding,
            BAR_WIDTH
        );

        assert_eq!(node.bounds().width, BAR_WIDTH);

        let bounds = node.children().iter().map(Node::bounds).collect::<Vec<_>>();

        // the left section starts at the left padding
        assert_eq!(bounds[0].x, padding.left);
        // the right section ends at the right padding, never at the left one
        assert_eq!(bounds[2].x + bounds[2].width, BAR_WIDTH - padding.right);
        // the centre section is centred on the content box
        assert_eq!(
            bounds[1].x + bounds[1].width / 2.0,
            padding.left + (BAR_WIDTH - padding.left - padding.right) / 2.0
        );
        // sections never overlap, so a point resolves to a single one
        assert!(bounds[0].x + bounds[0].width <= bounds[1].x);
        assert!(bounds[1].x + bounds[1].width <= bounds[2].x);
    }

    #[test]
    fn an_asymmetric_padding_still_anchors_the_right_section_to_the_right_edge() {
        let padding = Padding {
            top:    3.0,
            right:  30.0,
            bottom: 3.0,
            left:   4.0
        };
        let node = lay_out(
            [section(&[40.0]), section(&[80.0]), section(&[30.0, 50.0])],
            padding,
            BAR_WIDTH
        );

        let bounds = node.children().iter().map(Node::bounds).collect::<Vec<_>>();

        assert_eq!(bounds[0].x, padding.left);
        assert_eq!(bounds[2].x + bounds[2].width, BAR_WIDTH - padding.right);
    }

    #[test]
    fn a_point_inside_a_drawn_module_resolves_to_that_module() {
        let padding = Padding::from([3.0, 10.0]);
        let node = lay_out(
            [
                section(&[40.0, 60.0]),
                section(&[80.0]),
                section(&[30.0, 50.0, 20.0])
            ],
            padding,
            BAR_WIDTH
        );

        let layout = Layout::new(&node);

        for (section_index, section) in layout.children().enumerate() {
            for (module_index, module) in section.children().enumerate() {
                let bounds = module.bounds();

                for x in [
                    bounds.x + 0.5,
                    bounds.center_x(),
                    bounds.x + bounds.width - 0.5
                ] {
                    assert_eq!(
                        module_at(&node, x),
                        Some((section_index, module_index)),
                        "x={x} should resolve to module {section_index}.{module_index}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_rightmost_module_owns_the_pixels_up_to_the_right_padding() {
        let padding = Padding::from([3.0, 10.0]);
        let node = lay_out(
            [section(&[40.0]), section(&[80.0]), section(&[30.0, 50.0])],
            padding,
            BAR_WIDTH
        );

        let right_edge = BAR_WIDTH - padding.right;

        assert_eq!(section_at(&node, right_edge - 0.5), Some(2));
        assert_eq!(module_at(&node, right_edge - 0.5), Some((2, 1)));
        // beyond the padding nothing is hoverable any more
        assert_eq!(section_at(&node, right_edge + 0.5), None);
    }
}
