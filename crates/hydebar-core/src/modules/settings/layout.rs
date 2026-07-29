//! Pure edits of the module layout.
//!
//! Every operation of the module editor is a function from one layout to the
//! next, with no rendering and no file access in sight: the editor stays a thin
//! shell over rules that can be tested on their own.
//!
//! Edits address a single module rather than a whole island. The configuration
//! stores islands, which is what the bar draws, but an edit stated against an
//! island cannot say which of its modules it means.

mod flat;

pub use flat::Entry;
use flat::{flatten, rebuild};
use hydebar_proto::config::{ModuleDef, ModuleName, Modules};

/// Region of the bar a list of modules belongs to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    Left,
    Center,
    Right
}

impl Section {
    /// Every section, in the order the editor lists them.
    pub const ALL: [Section; 3] = [Section::Left, Section::Center, Section::Right];

    /// Name shown above the row of this section.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Section::Left => "Left",
            Section::Center => "Center",
            Section::Right => "Right"
        }
    }

    /// Section on the left of this one, if any.
    #[must_use]
    pub const fn before(self) -> Option<Self> {
        match self {
            Section::Left => None,
            Section::Center => Some(Section::Left),
            Section::Right => Some(Section::Center)
        }
    }

    /// Section on the right of this one, if any.
    #[must_use]
    pub const fn after(self) -> Option<Self> {
        match self {
            Section::Left => Some(Section::Center),
            Section::Center => Some(Section::Right),
            Section::Right => None
        }
    }

    /// Islands of this section.
    #[must_use]
    pub fn islands(self, modules: &Modules) -> &Vec<ModuleDef> {
        match self {
            Section::Left => &modules.left,
            Section::Center => &modules.center,
            Section::Right => &modules.right
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
            Section::Left => modules.left = islands,
            Section::Center => modules.center = islands,
            Section::Right => modules.right = islands
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
        section: Section,
        module:  ModuleName
    }
}

impl LayoutEdit {
    /// Slot this edit acts on, if it acts on one.
    #[must_use]
    pub fn slot(&self) -> Option<Slot> {
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

/// Takes the module out of its section, leaving the rest joined sensibly.
///
/// A module that led an island hands the island over to the module after it, so
/// removing the head of a group does not silently dissolve the group.
fn take(entries: &mut Vec<Entry>, index: usize) -> Option<Entry> {
    if index >= entries.len() {
        return None;
    }

    let taken = entries.remove(index);

    if !taken.joined
        && let Some(next) = entries.get_mut(index)
    {
        next.joined = false;
    }

    Some(taken)
}

/// Returns the layout `edit` produces from `modules`.
///
/// An edit that cannot apply, moving the first module earlier for instance,
/// leaves the layout untouched: the editor may offer every button
/// unconditionally and still never produce a broken layout.
#[must_use]
pub fn apply(modules: &Modules, edit: &LayoutEdit) -> Modules {
    let mut next = modules.clone();

    match edit {
        LayoutEdit::MoveEarlier(slot) => {
            let mut entries = slot.section.entries(modules);

            if slot.index > 0 && slot.index < entries.len() {
                entries.swap(slot.index - 1, slot.index);
                let first_joined = entries[0].joined;
                entries[0].joined = false;
                entries[slot.index].joined |= first_joined && slot.index == 1;
                slot.section.store(&mut next, &entries);
            }
        }
        LayoutEdit::MoveLater(slot) => {
            let mut entries = slot.section.entries(modules);

            if slot.index + 1 < entries.len() {
                entries.swap(slot.index, slot.index + 1);
                entries[0].joined = false;
                slot.section.store(&mut next, &entries);
            }
        }
        LayoutEdit::MoveToPreviousSection(slot) => {
            let Some(target) = slot.section.before() else {
                return next;
            };

            let mut entries = slot.section.entries(modules);

            if let Some(mut moved) = take(&mut entries, slot.index) {
                moved.joined = false;
                slot.section.store(&mut next, &entries);

                let mut into = target.entries(&next);
                into.push(moved);
                target.store(&mut next, &into);
            }
        }
        LayoutEdit::MoveToNextSection(slot) => {
            let Some(target) = slot.section.after() else {
                return next;
            };

            let mut entries = slot.section.entries(modules);

            if let Some(mut moved) = take(&mut entries, slot.index) {
                moved.joined = false;
                slot.section.store(&mut next, &entries);

                let mut into = target.entries(&next);
                into.insert(0, moved);

                if let Some(second) = into.get_mut(1) {
                    second.joined = false;
                }

                target.store(&mut next, &into);
            }
        }
        LayoutEdit::ToggleJoin(slot) => {
            let mut entries = slot.section.entries(modules);

            if slot.index > 0 && slot.index < entries.len() {
                entries[slot.index].joined = !entries[slot.index].joined;
                slot.section.store(&mut next, &entries);
            }
        }
        LayoutEdit::Remove(slot) => {
            let mut entries = slot.section.entries(modules);

            if take(&mut entries, slot.index).is_some() {
                slot.section.store(&mut next, &entries);
            }
        }
        LayoutEdit::Add {
            section,
            module
        } => {
            let mut entries = section.entries(modules);
            entries.push(Entry {
                module: module.clone(),
                joined: false
            });
            section.store(&mut next, &entries);
        }
    }

    next
}

/// Modules already placed somewhere on the bar.
#[must_use]
pub fn placed(modules: &Modules) -> Vec<ModuleName> {
    Section::ALL
        .into_iter()
        .flat_map(|section| section.entries(modules))
        .map(|entry| entry.module)
        .collect()
}

/// Modules offered by the editor that are not on the bar yet.
///
/// `custom` names the modules the user defined themselves, so they can be
/// placed like any built in one.
#[must_use]
pub fn available(modules: &Modules, custom: &[String]) -> Vec<ModuleName> {
    let placed = placed(modules);

    ModuleName::BUILT_IN
        .into_iter()
        .chain(custom.iter().cloned().map(ModuleName::Custom))
        .filter(|module| !placed.contains(module))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(left: Vec<ModuleDef>) -> Modules {
        Modules {
            left,
            center: Vec::new(),
            right: Vec::new()
        }
    }

    fn slot(index: usize) -> Slot {
        Slot {
            section: Section::Left,
            index
        }
    }

    fn names(modules: &Modules, section: Section) -> Vec<String> {
        section
            .entries(modules)
            .into_iter()
            .map(|entry| {
                format!(
                    "{}{}",
                    if entry.joined { "+" } else { "" },
                    entry.module.as_str()
                )
            })
            .collect()
    }

    #[test]
    fn a_module_moves_inside_its_section() {
        let modules = layout(vec![
            ModuleDef::Single(ModuleName::Clock),
            ModuleDef::Single(ModuleName::Tray),
        ]);

        let moved = apply(&modules, &LayoutEdit::MoveLater(slot(0)));

        assert_eq!(names(&moved, Section::Left), vec!["Tray", "Clock"]);
    }

    #[test]
    fn a_move_beyond_the_ends_changes_nothing() {
        let modules = layout(vec![ModuleDef::Single(ModuleName::Clock)]);

        for edit in [
            LayoutEdit::MoveEarlier(slot(0)),
            LayoutEdit::MoveLater(slot(0)),
            LayoutEdit::MoveEarlier(slot(9))
        ] {
            assert_eq!(names(&apply(&modules, &edit), Section::Left), vec!["Clock"]);
        }
    }

    #[test]
    fn a_single_module_leaves_its_island() {
        let modules = layout(vec![ModuleDef::Group(vec![
            ModuleName::Clock,
            ModuleName::Tray,
            ModuleName::Battery,
        ])]);

        let split = apply(&modules, &LayoutEdit::ToggleJoin(slot(1)));

        assert_eq!(
            split.left,
            vec![
                ModuleDef::Single(ModuleName::Clock),
                ModuleDef::Group(vec![ModuleName::Tray, ModuleName::Battery])
            ]
        );
    }

    #[test]
    fn a_module_joins_the_island_above() {
        let modules = layout(vec![
            ModuleDef::Single(ModuleName::Clock),
            ModuleDef::Single(ModuleName::Tray),
        ]);

        let joined = apply(&modules, &LayoutEdit::ToggleJoin(slot(1)));

        assert_eq!(
            joined.left,
            vec![ModuleDef::Group(vec![ModuleName::Clock, ModuleName::Tray])]
        );
    }

    #[test]
    fn the_first_module_of_a_section_cannot_join_upwards() {
        let modules = layout(vec![ModuleDef::Single(ModuleName::Clock)]);

        assert_eq!(
            apply(&modules, &LayoutEdit::ToggleJoin(slot(0))).left,
            modules.left
        );
    }

    #[test]
    fn removing_the_head_of_an_island_keeps_the_island() {
        let modules = layout(vec![ModuleDef::Group(vec![
            ModuleName::Clock,
            ModuleName::Tray,
            ModuleName::Battery,
        ])]);

        let removed = apply(&modules, &LayoutEdit::Remove(slot(0)));

        assert_eq!(
            removed.left,
            vec![ModuleDef::Group(vec![
                ModuleName::Tray,
                ModuleName::Battery
            ])]
        );
    }

    #[test]
    fn a_module_travels_between_sections() {
        let modules = Modules {
            left:   vec![ModuleDef::Single(ModuleName::Clock)],
            center: Vec::new(),
            right:  Vec::new()
        };

        let moved = apply(
            &modules,
            &LayoutEdit::MoveToNextSection(Slot {
                section: Section::Left,
                index:   0
            })
        );

        assert!(moved.left.is_empty());
        assert_eq!(names(&moved, Section::Center), vec!["Clock"]);

        let back = apply(
            &moved,
            &LayoutEdit::MoveToPreviousSection(Slot {
                section: Section::Center,
                index:   0
            })
        );

        assert_eq!(names(&back, Section::Left), vec!["Clock"]);
    }

    #[test]
    fn the_outer_sections_have_no_neighbour_beyond_them() {
        assert_eq!(Section::Left.before(), None);
        assert_eq!(Section::Right.after(), None);
    }

    #[test]
    fn a_module_moved_out_of_an_island_arrives_on_its_own() {
        let modules = Modules {
            left:   vec![ModuleDef::Group(vec![ModuleName::Clock, ModuleName::Tray])],
            center: Vec::new(),
            right:  Vec::new()
        };

        let moved = apply(
            &modules,
            &LayoutEdit::MoveToNextSection(Slot {
                section: Section::Left,
                index:   1
            })
        );

        assert_eq!(names(&moved, Section::Left), vec!["Clock"]);
        assert_eq!(names(&moved, Section::Center), vec!["Tray"]);
    }

    #[test]
    fn an_added_module_lands_at_the_end_on_its_own() {
        let modules = layout(vec![ModuleDef::Group(vec![
            ModuleName::Clock,
            ModuleName::Tray,
        ])]);

        let added = apply(
            &modules,
            &LayoutEdit::Add {
                section: Section::Left,
                module:  ModuleName::Battery
            }
        );

        assert_eq!(
            names(&added, Section::Left),
            vec!["Clock", "+Tray", "Battery"]
        );
    }

    #[test]
    fn the_available_list_skips_what_is_already_placed() {
        let modules = Modules {
            left:   vec![ModuleDef::Single(ModuleName::Clock)],
            center: vec![ModuleDef::Group(vec![ModuleName::Workspaces])],
            right:  Vec::new()
        };

        let available = available(&modules, &["power".to_owned()]);

        assert!(!available.contains(&ModuleName::Clock));
        assert!(!available.contains(&ModuleName::Workspaces));
        assert!(available.contains(&ModuleName::Tray));
        assert!(available.contains(&ModuleName::Custom("power".to_owned())));
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
