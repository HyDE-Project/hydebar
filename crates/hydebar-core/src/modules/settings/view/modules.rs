//! Module editor page of the settings window.
//!
//! The page is a small picture of the bar: three sections, the modules
//! in the order they are drawn, islands boxed together. At rest
//! it says only what the bar looks like. Picking a module opens
//! a card that names it, says where it sits, and offers its
//! actions in labelled groups, so the page stays short
//! while every action is spelled out when it matters.
//!
//! Reordering is done with buttons rather than by dragging: a drag
//! needs a pointer held along a path, which a keyboard, a
//! trackpad in accessibility mode and a voice control cannot
//! produce.
//!
//! One folder, four rooms: [`page`] assembles the page, [`detail`]
//! draws the card of the picked module, [`catalogue`] offers the
//! modules that are not on the bar yet and [`metrics`] measures the
//! room the page asks for. The root holds the island grouping the
//! rooms share.

mod catalogue;
mod detail;
mod metrics;
mod page;

pub(super) use metrics::{desired_height, desired_width};
#[cfg(test)]
pub(super) use metrics::rows;
pub(super) use page::view;

use crate::modules::settings::layout::Entry;

/// Groups the entries of a section into the islands they form.
fn islands(entries: &[Entry]) -> Vec<Vec<usize>> {
    let mut islands: Vec<Vec<usize>> = Vec::new();

    for (index, entry) in entries.iter().enumerate() {
        match (entry.joined, islands.last_mut()) {
            (true, Some(island)) => island.push(index),
            _ => islands.push(vec![index])
        }
    }

    islands
}

/// Island the module at `index` belongs to, counted from one.
fn island_of(entries: &[Entry], index: usize) -> usize {
    islands(entries)
        .into_iter()
        .position(|island| island.contains(&index))
        .map_or(1, |position| position + 1)
}

#[cfg(test)]
mod tests {
    use hydebar_proto::config::{ModuleDef, ModuleName, Modules};

    use super::*;
    use crate::modules::settings::layout::Section;

    fn entries(left: Vec<ModuleDef>) -> Vec<Entry> {
        Section::Left.entries(&Modules {
            left,
            center: Vec::new(),
            right: Vec::new()
        })
    }

    #[test]
    fn neighbouring_joined_modules_form_one_island() {
        let section = entries(vec![
            ModuleDef::Group(vec![ModuleName::Clock, ModuleName::Tray]),
            ModuleDef::Single(ModuleName::Battery),
        ]);

        assert_eq!(islands(&section), vec![vec![0, 1], vec![2]]);
    }

    #[test]
    fn a_section_of_singles_is_a_row_of_islands() {
        let section = entries(vec![
            ModuleDef::Single(ModuleName::Clock),
            ModuleDef::Single(ModuleName::Tray),
        ]);

        assert_eq!(islands(&section), vec![vec![0], vec![1]]);
    }

    #[test]
    fn an_empty_section_has_no_islands() {
        assert!(islands(&[]).is_empty());
    }

    #[test]
    fn a_module_knows_which_island_it_sits_in() {
        let section = entries(vec![
            ModuleDef::Group(vec![ModuleName::Clock, ModuleName::Tray]),
            ModuleDef::Single(ModuleName::Battery),
        ]);

        assert_eq!(island_of(&section, 0), 1);
        assert_eq!(island_of(&section, 1), 1);
        assert_eq!(island_of(&section, 2), 2);
    }

    #[test]
    fn a_missing_module_is_reported_as_the_first_island() {
        assert_eq!(island_of(&[], 3), 1);
    }
}
