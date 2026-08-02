//! Painting the pills under wherever the modules currently are, then the
//! modules themselves.

use iced::{
    Rectangle, Vector,
    core::{Layout, mouse, renderer, widget::Tree}
};

use super::builder::Archipelago;

/// Draws the fused pills, then every module at its travelled position.
///
/// Pills of modules that draw near each other fuse into one island and
/// part again as they pass; a pill fades in with the strongest arrival
/// wave among its tenants.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors the upstream widget draw signature"
)]
pub(super) fn draw<Message, Theme, Renderer>(
    strip: &Archipelago<'_, Message, Theme, Renderer>,
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
    let bounds = layout.bounds();

    let (offsets, spans) = {
        let mut memo = strip.memo.borrow_mut();
        let mut offsets = Vec::with_capacity(strip.children.len());
        let mut spans: Vec<(f32, f32)> = Vec::with_capacity(strip.children.len());

        for (index, child_layout) in layout.children().enumerate() {
            let child_bounds = child_layout.bounds();

            let offset = strip.offset(memo.from_map(), index, child_bounds.x);
            let current = child_bounds.x + offset;

            memo.record(strip.keys[index], current);

            offsets.push(offset);
            spans.push((
                current - strip.pad_x,
                current + child_bounds.width + strip.pad_x
            ));
        }

        (offsets, spans)
    };

    if let Some(paint) = (strip.paint)(theme) {
        let mut clusters: Vec<(f32, f32, f32)> = Vec::new();
        let mut sorted: Vec<(f32, f32, f32)> = spans
            .iter()
            .zip(strip.arrivals.iter())
            .map(|(&(start, end), &arrival)| (start, end, arrival))
            .collect();
        sorted.sort_by(|a, b| a.0.total_cmp(&b.0));

        let fuse = strip.island_gap * 0.5;

        for (start, end, arrival) in sorted {
            match clusters.last_mut() {
                Some((_, tail, shown)) if start - *tail <= fuse => {
                    *tail = tail.max(end);
                    *shown = shown.max(arrival);
                }
                _ => clusters.push((start, end, arrival))
            }
        }

        for (start, end, shown) in clusters {
            if shown <= f32::EPSILON {
                continue;
            }

            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x:      start,
                        y:      bounds.y,
                        width:  end - start,
                        height: bounds.height
                    },
                    border: paint.border,
                    shadow: paint.shadow,
                    ..renderer::Quad::default()
                },
                paint.background.scale_alpha(shown)
            );
        }
    }

    for (index, ((child, sub_tree), child_layout)) in strip
        .children
        .iter()
        .zip(tree.children.iter())
        .zip(layout.children())
        .enumerate()
    {
        if offsets[index].abs() < f32::EPSILON {
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
            renderer.with_translation(Vector::new(offsets[index], 0.0), |renderer| {
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
