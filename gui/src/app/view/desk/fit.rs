//! Sizing the writing so the deepest column ends on the screen.
//!
//! The canvas says everything the bar knows, and a machine that knows a lot
//! has more to say than a column of one size has room for. A column that runs
//! off the bottom edge does not merely lose its last block — it loses it
//! silently, and a status screen that hides a reading is worse than one that
//! never offered it.
//!
//! So the ink is sized to the deepest column rather than fixed: the room every
//! block will take is worked out from the same figures that reserve it, and
//! the writing is stepped down until the whole of it fits. It steps down only
//! so far — writing nobody can read across a room is not a fit either — and a
//! canvas that already fits is left at the size the theme asked for.

use hydebar_core::config::ModuleName;
use iced::SurfaceId as Id;

use super::{super::super::state::App, blocks};

/// Smallest share of the theme's own size the writing is stepped down to.
///
/// Past this the canvas stops being readable from across the room, which is
/// the one thing it is for; a machine with more to say than this can hold
/// keeps its size and lets the last block run off the edge.
const FLOOR: f32 = 0.62;

/// Steps the writing down until the deepest column fits, or as far as it goes.
///
/// Takes the room the columns need at the theme's own size and the height the
/// canvas has to fill; hands back the size the whole canvas is written at.
pub(super) fn ink_size(app: &App, id: Id, room: f32) -> f32 {
    let size = app.appearance().font_size_px();
    let deepest = deepest_column(app, id, size);

    if deepest <= room || deepest <= 0.0 {
        return size;
    }

    size * (room / deepest).clamp(FLOOR, 1.0)
}

/// The room the tallest of the three columns asks for, at the given size.
fn deepest_column(app: &App, id: Id, size: f32) -> f32 {
    let (left, centre, right) = App::desk_columns(&app.config.modules);

    [&left, &centre, &right]
        .into_iter()
        .map(|order| column_room(app, id, order, size))
        .fold(0.0_f32, f32::max)
}

/// The room one column asks for: every unit, and the gap between them.
fn column_room(app: &App, id: Id, order: &[&ModuleName], size: f32) -> f32 {
    let ink = blocks::Ink {
        value: app.theme_cache.palette().text,
        size
    };
    let between = size * 1.8;
    let units = order
        .iter()
        .filter(|unit| app.desk_island_exists(unit, id))
        .count();

    if units == 0 {
        return 0.0;
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "a column holds a handful of units"
    )]
    let gaps = (units - 1) as f32 * between;

    order
        .iter()
        .map(|unit| app.desk_unit_room(unit, ink))
        .sum::<f32>()
        + gaps
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::{super::super::super::state::test_support::test_app_with, *};

    fn app() -> App {
        test_app_with(|config| config.desk.enabled = true)
    }

    fn surface() -> Id {
        Id::unique()
    }

    #[test]
    fn a_canvas_with_room_to_spare_keeps_the_size_the_theme_asked_for() {
        let app = app();

        assert_eq!(
            ink_size(&app, surface(), 100_000.0),
            app.appearance().font_size_px()
        );
    }

    #[test]
    fn a_canvas_with_no_room_is_stepped_down_but_only_so_far() {
        let app = app();
        let full = app.appearance().font_size_px();

        let squeezed = ink_size(&app, surface(), 1.0);

        assert!(
            squeezed < full,
            "a column that cannot fit is written smaller"
        );
        assert_eq!(
            squeezed,
            full * FLOOR,
            "and never smaller than a person can read"
        );
    }

    #[test]
    fn the_step_falls_as_the_room_does() {
        let app = app();
        let roomy = ink_size(&app, surface(), 4000.0);
        let tight = ink_size(&app, surface(), 1500.0);

        assert!(roomy >= tight, "less room, smaller writing");
    }
}
