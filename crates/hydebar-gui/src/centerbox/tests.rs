use hydebar_core::position_button::position_button;
use iced::{
    Alignment, Element, Length, Padding, Point, Size,
    advanced::{
        layout::{Layout, Limits, Node},
        widget::Tree
    },
    widget::{Space, container, row}
};

use super::Centerbox;

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
