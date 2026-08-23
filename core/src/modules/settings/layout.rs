//! Pure edits of the module layout.
//!
//! Every operation of the module editor is a function from one layout to
//! the next, with no rendering and no file access in sight: the editor
//! stays a thin shell over rules that can be tested on their own.
//!
//! Edits address a single module rather than a whole island. The
//! configuration stores islands, which is what the bar draws, but an
//! edit stated against an island cannot say which of its modules it
//! means.
//!
//! One folder, three rooms: [`flat`] maps islands onto rows and back,
//! [`edits`] applies one edit to a layout and [`catalogue`] says which
//! modules can still be added. The root holds the sections, slots and
//! edits the rooms share.

mod catalogue;
mod edits;
mod flat;

pub use catalogue::available;
pub use edits::apply;
pub use flat::Entry;
use flat::{flatten, rebuild};
use hydebar_proto::config::{ModuleDef, ModuleName, Modules};

/// Region of the bar a list of modules belongs to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Section {
    /// The section against the leading edge.
    #[default]
    Left,
    /// The middle section.
    Center,
    /// The section against the trailing edge.
    Right
}

impl Section {
    /// Every section, in the order the editor lists them.
    pub const ALL: [Self; 3] = [Self::Left, Self::Center, Self::Right];

    /// Name shown above the row of this section.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Center => "Center",
            Self::Right => "Right"
        }
    }

    /// Section on the left of this one, if any.
    #[must_use]
    pub const fn before(self) -> Option<Self> {
        match self {
            Self::Left => None,
            Self::Center => Some(Self::Left),
            Self::Right => Some(Self::Center)
        }
    }

    /// Section on the right of this one, if any.
    #[must_use]
    pub const fn after(self) -> Option<Self> {
        match self {
            Self::Left => Some(Self::Center),
            Self::Center => Some(Self::Right),
            Self::Right => None
        }
    }

    /// Islands of this section.
    #[must_use]
    pub const fn islands(self, modules: &Modules) -> &Vec<ModuleDef> {
        match self {
            Self::Left => &modules.left,
            Self::Center => &modules.center,
            Self::Right => &modules.right
        }
    }

    /// Modules of this section, one entry each.
    #[must_use]
    pub fn entries(self, modules: &Modules) -> Vec<Entry> {
        flatten(self.islands(modules))
    }

    /// Replaces the modules of this section.
    fn store(self, modules: &mut Modules, entries: &[Entry]) {
        let islands = rebuild(entries);

        match self {
            Self::Left => modules.left = islands,
            Self::Center => modules.center = islands,
            Self::Right => modules.right = islands
        }
    }
}

/// Where an edit applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// Section the module sits in.
    pub section: Section,
    /// Position of the module inside its section.
    pub index:   usize
}

/// A single change the module editor can make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutEdit {
    /// Swap the module with the one before it.
    MoveEarlier(Slot),
    /// Swap the module with the one after it.
    MoveLater(Slot),
    /// Move the module to the end of the section on the left.
    MoveToPreviousSection(Slot),
    /// Move the module to the start of the section on the right.
    MoveToNextSection(Slot),
    /// Join the module to the island above, or break it out of one.
    ToggleJoin(Slot),
    /// Drop the module from the bar.
    Remove(Slot),
    /// Append a module to a section.
    Add {
        /// Which of the three sections it stands in.
        section: Section,
        /// Which module it is.
        module:  ModuleName
    }
}

impl LayoutEdit {
    /// Slot this edit acts on, if it acts on one.
    #[must_use]
    pub const fn slot(&self) -> Option<Slot> {
        match self {
            Self::MoveEarlier(slot)
            | Self::MoveLater(slot)
            | Self::MoveToPreviousSection(slot)
            | Self::MoveToNextSection(slot)
            | Self::ToggleJoin(slot)
            | Self::Remove(slot) => Some(*slot),
            Self::Add {
                ..
            } => None
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn slot(index: usize) -> Slot {
        Slot {
            section: Section::Left,
            index
        }
    }

    #[test]
    fn the_outer_sections_have_no_neighbour_beyond_them() {
        assert_eq!(Section::Left.before(), None);
        assert_eq!(Section::Right.after(), None);
    }

    #[test]
    fn every_edit_but_adding_names_the_slot_it_acts_on() {
        assert_eq!(LayoutEdit::Remove(slot(2)).slot(), Some(slot(2)));
        assert_eq!(
            LayoutEdit::Add {
                section: Section::Left,
                module:  ModuleName::Clock
            }
            .slot(),
            None
        );
    }
}
