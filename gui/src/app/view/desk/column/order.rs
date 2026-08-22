//! What stands where in a column, and what one place holds.

use hydebar_core::config::{ModuleDef, ModuleName};

use super::super::super::super::state::App;

impl App {
    /// The units of one section, in the order the canvas stands them in.
    ///
    /// A unit is one module. The modules of a group shared one pill on the
    /// strip and each carries a pill of its own to its place on the canvas:
    /// what parts is the icons, and the backing goes with both of them.
    ///
    /// The rule is the distance from the middle of the strip: a unit that
    /// stood near the centre stands high on the canvas, and one that stood at
    /// an edge reaches for the corner below it. The centre section already
    /// reads outwards from the middle; the left one reads towards it, so it
    /// is turned around.
    pub(crate) fn desk_order(
        section: &[ModuleDef],
        reads_towards_the_centre: bool
    ) -> Vec<&ModuleName> {
        let mut order: Vec<&ModuleName> = section.iter().flat_map(Self::members).collect();

        if reads_towards_the_centre {
            order.reverse();
        }

        order
    }

    /// The modules one unit carries.
    pub(crate) fn members(unit: &ModuleDef) -> Vec<&ModuleName> {
        match unit {
            ModuleDef::Single(module) => vec![module],
            ModuleDef::Group(group) => group.iter().collect()
        }
    }
}
