//! Breaking a column that is too deep into runs that stand side by side.
//!
//! A section with a dozen modules has more to say than the height of a screen
//! holds, and a screen is far wider than one column of readings needs. So a
//! column that would run off the bottom is broken instead: the blocks are
//! dealt into runs of the height there is, and the runs stand beside one
//! another. Nothing is dropped, nothing is shrunk, and the order is kept —
//! read the first run down, then the next, the way a page of columns is read.

use hydebar_core::config::ModuleName;
use iced::SurfaceId as Id;

use super::super::{super::super::state::App, blocks::Ink};

/// How many runs one section is ever broken into.
///
/// Three thirds of a screen, each broken in two, is six runs across — which
/// is as many as the width holds before the readings stop having a measure.
const MOST: usize = 2;

impl App {
    /// Deals the units of a section into the runs they are drawn in.
    ///
    /// Every run is a list of places in `order`; a section that fits in the
    /// room it has comes back as one run holding all of them. Blocks that
    /// draw nothing are left out, because they take no place in the column.
    pub(in crate::app::view::desk) fn desk_runs(
        &self,
        order: &[&ModuleName],
        id: Id,
        ink: Ink,
        room: f32
    ) -> Vec<Vec<usize>> {
        let drawn: Vec<usize> = order
            .iter()
            .enumerate()
            .filter(|(_, unit)| self.desk_island_exists(unit, id))
            .map(|(at, _)| at)
            .collect();

        if drawn.is_empty() {
            return Vec::new();
        }

        let between = ink.size * 1.8;
        let mut runs: Vec<Vec<usize>> = vec![Vec::new()];
        let mut standing = 0.0_f32;

        for at in drawn {
            let takes = self.desk_unit_room(order[at], ink);
            let with_gap = if runs[runs.len() - 1].is_empty() {
                takes
            } else {
                takes + between
            };

            if standing + with_gap > room && runs.len() < MOST && !runs[runs.len() - 1].is_empty()
            {
                runs.push(vec![at]);
                standing = takes;
                continue;
            }

            standing += with_gap;
            runs.last_mut().unwrap_or(&mut Vec::new()).push(at);
        }

        runs.retain(|run| !run.is_empty());
        runs
    }

    /// The room the deepest run of a section stands in, once it is broken.
    pub(in crate::app::view::desk) fn desk_section_room(
        &self,
        order: &[&ModuleName],
        id: Id,
        ink: Ink,
        room: f32
    ) -> f32 {
        let between = ink.size * 1.8;

        self.desk_runs(order, id, ink, room)
            .into_iter()
            .map(|run| {
                #[expect(clippy::cast_precision_loss, reason = "a run holds a handful of units")]
                let gaps = (run.len().max(1) - 1) as f32 * between;

                run.into_iter()
                    .map(|at| self.desk_unit_room(order[at], ink))
                    .sum::<f32>()
                    + gaps
            })
            .fold(0.0_f32, f32::max)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_proto::config::ModuleDef;

    use super::{super::super::super::super::state::test_support::test_app_with, *};

    fn ink() -> Ink {
        Ink {
            value: iced::Color::WHITE,
            size:  14.0
        }
    }

    fn app() -> App {
        test_app_with(|config| {
            config.desk.enabled = true;
            config.modules.left = vec![
                ModuleDef::Single(ModuleName::Clock),
                ModuleDef::Single(ModuleName::Memory),
                ModuleDef::Single(ModuleName::Cpu),
            ];
        })
    }

    #[test]
    fn a_section_that_fits_comes_back_as_one_run() {
        let app = app();
        let (left, _, _) = App::desk_columns(&app.config.modules);

        let runs = app.desk_runs(&left, Id::unique(), ink(), 100_000.0);

        assert_eq!(runs.len(), 1, "nothing is broken that fits");
        assert_eq!(runs[0].len(), left.len());
    }

    #[test]
    fn a_section_that_does_not_fit_is_dealt_into_runs_in_order() {
        let app = app();
        let (left, _, _) = App::desk_columns(&app.config.modules);

        let runs = app.desk_runs(&left, Id::unique(), ink(), 1.0);

        assert!(runs.len() > 1, "a column too deep stands beside itself");

        let dealt: Vec<usize> = runs.concat();
        let mut ordered = dealt.clone();
        ordered.sort_unstable();

        assert_eq!(dealt, ordered, "the order of the section is kept");
        assert_eq!(dealt.len(), left.len(), "and nothing is dropped");
    }

    #[test]
    fn a_section_is_never_broken_past_the_runs_the_width_holds() {
        let app = app();
        let (left, _, _) = App::desk_columns(&app.config.modules);

        assert!(app.desk_runs(&left, Id::unique(), ink(), 1.0).len() <= MOST);
    }
}
