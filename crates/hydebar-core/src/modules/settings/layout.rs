//! Pure edits of the module layout.
//!
//! Every operation of the module editor is a function from one layout to the
//! next, with no rendering and no file access in sight: the editor stays a thin
//! shell over rules that can be tested on their own.

use hydebar_proto::config::{ModuleDef, ModuleName, Modules};

/// Region of the bar a list of modules belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Left,
    Center,
    Right
}

impl Section {
    /// Every section, in the order the editor lists them.
    pub const ALL: [Section; 3] = [Section::Left, Section::Center, Section::Right];

    /// Name shown above the column of this section.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Section::Left => "Left",
            Section::Center => "Center",
            Section::Right => "Right"
        }
    }

    /// Section the entries move to when pushed sideways.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Section::Left => Section::Center,
            Section::Center => Section::Right,
            Section::Right => Section::Left
        }
    }

    /// Entries of this section.
    fn entries(self, modules: &Modules) -> &Vec<ModuleDef> {
        match self {
            Section::Left => &modules.left,
            Section::Center => &modules.center,
            Section::Right => &modules.right
        }
    }

    /// Entries of this section, for editing.
    fn entries_mut(self, modules: &mut Modules) -> &mut Vec<ModuleDef> {
        match self {
            Section::Left => &mut modules.left,
            Section::Center => &mut modules.center,
            Section::Right => &mut modules.right
        }
    }
}

/// A single change the module editor can make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutEdit {
    /// Swap the entry with the one before it.
    MoveUp { section: Section, index: usize },
    /// Swap the entry with the one after it.
    MoveDown { section: Section, index: usize },
    /// Take the entry out of its section and append it to the next one.
    MoveToNextSection { section: Section, index: usize },
    /// Drop the entry from the bar.
    Remove { section: Section, index: usize },
    /// Merge the entry with the one after it into a single island.
    GroupWithNext { section: Section, index: usize },
    /// Split an island back into standalone entries.
    Ungroup { section: Section, index: usize },
    /// Append a module to the end of a section.
    Add {
        section: Section,
        module:  ModuleName
    }
}

/// Flattens an entry into the modules it holds.
fn members(entry: &ModuleDef) -> Vec<ModuleName> {
    match entry {
        ModuleDef::Single(name) => vec![name.clone()],
        ModuleDef::Group(group) => group.clone()
    }
}

/// Builds the entry holding `members`, collapsing a single member back into a
/// standalone entry.
fn entry_from(members: Vec<ModuleName>) -> Option<ModuleDef> {
    match members.len() {
        0 => None,
        1 => members.into_iter().next().map(ModuleDef::Single),
        _ => Some(ModuleDef::Group(members))
    }
}

/// Returns the layout `edit` produces from `modules`.
///
/// An edit that cannot apply, moving the first entry up for instance, leaves
/// the layout untouched: the editor may offer every button unconditionally and
/// still never produce a broken layout.
#[must_use]
pub fn apply(modules: &Modules, edit: &LayoutEdit) -> Modules {
    let mut next = modules.clone();

    match edit {
        LayoutEdit::MoveUp {
            section,
            index
        } => {
            let entries = section.entries_mut(&mut next);

            if *index > 0 && *index < entries.len() {
                entries.swap(*index - 1, *index);
            }
        }
        LayoutEdit::MoveDown {
            section,
            index
        } => {
            let entries = section.entries_mut(&mut next);

            if index + 1 < entries.len() {
                entries.swap(*index, *index + 1);
            }
        }
        LayoutEdit::MoveToNextSection {
            section,
            index
        } => {
            let entries = section.entries_mut(&mut next);

            if *index < entries.len() {
                let entry = entries.remove(*index);
                section.next().entries_mut(&mut next).push(entry);
            }
        }
        LayoutEdit::Remove {
            section,
            index
        } => {
            let entries = section.entries_mut(&mut next);

            if *index < entries.len() {
                entries.remove(*index);
            }
        }
        LayoutEdit::GroupWithNext {
            section,
            index
        } => {
            let entries = section.entries_mut(&mut next);

            if index + 1 < entries.len() {
                let following = entries.remove(index + 1);
                let mut merged = members(&entries[*index]);
                merged.extend(members(&following));

                if let Some(entry) = entry_from(merged) {
                    entries[*index] = entry;
                }
            }
        }
        LayoutEdit::Ungroup {
            section,
            index
        } => {
            let entries = section.entries_mut(&mut next);

            if *index < entries.len()
                && let ModuleDef::Group(group) = entries[*index].clone()
            {
                entries.splice(
                    *index..=*index,
                    group.into_iter().map(ModuleDef::Single).collect::<Vec<_>>()
                );
            }
        }
        LayoutEdit::Add {
            section,
            module
        } => {
            section
                .entries_mut(&mut next)
                .push(ModuleDef::Single(module.clone()));
        }
    }

    next
}

/// Modules already placed somewhere on the bar.
#[must_use]
pub fn placed(modules: &Modules) -> Vec<ModuleName> {
    Section::ALL
        .into_iter()
        .flat_map(|section| section.entries(modules).iter().flat_map(members))
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

    fn single(name: ModuleName) -> ModuleDef {
        ModuleDef::Single(name)
    }

    #[test]
    fn an_entry_moves_up_and_down_inside_its_section() {
        let modules = layout(vec![single(ModuleName::Clock), single(ModuleName::Battery)]);

        let moved = apply(
            &modules,
            &LayoutEdit::MoveUp {
                section: Section::Left,
                index:   1
            }
        );
        assert_eq!(
            moved.left,
            vec![single(ModuleName::Battery), single(ModuleName::Clock)]
        );

        let back = apply(
            &moved,
            &LayoutEdit::MoveDown {
                section: Section::Left,
                index:   0
            }
        );
        assert_eq!(back.left, modules.left);
    }

    #[test]
    fn a_move_beyond_the_ends_changes_nothing() {
        let modules = layout(vec![single(ModuleName::Clock)]);

        for edit in [
            LayoutEdit::MoveUp {
                section: Section::Left,
                index:   0
            },
            LayoutEdit::MoveDown {
                section: Section::Left,
                index:   0
            },
            LayoutEdit::MoveUp {
                section: Section::Left,
                index:   9
            }
        ] {
            assert_eq!(apply(&modules, &edit).left, modules.left);
        }
    }

    #[test]
    fn an_entry_travels_to_the_next_section() {
        let modules = layout(vec![single(ModuleName::Clock)]);

        let moved = apply(
            &modules,
            &LayoutEdit::MoveToNextSection {
                section: Section::Left,
                index:   0
            }
        );

        assert!(moved.left.is_empty());
        assert_eq!(moved.center, vec![single(ModuleName::Clock)]);
    }

    #[test]
    fn the_last_section_wraps_back_to_the_first() {
        assert_eq!(Section::Right.next(), Section::Left);
    }

    #[test]
    fn two_neighbours_merge_into_one_island() {
        let modules = layout(vec![single(ModuleName::Clock), single(ModuleName::Battery)]);

        let grouped = apply(
            &modules,
            &LayoutEdit::GroupWithNext {
                section: Section::Left,
                index:   0
            }
        );

        assert_eq!(
            grouped.left,
            vec![ModuleDef::Group(vec![
                ModuleName::Clock,
                ModuleName::Battery
            ])]
        );
    }

    #[test]
    fn grouping_absorbs_an_existing_island() {
        let modules = layout(vec![
            ModuleDef::Group(vec![ModuleName::Clock, ModuleName::Battery]),
            single(ModuleName::Tray),
        ]);

        let grouped = apply(
            &modules,
            &LayoutEdit::GroupWithNext {
                section: Section::Left,
                index:   0
            }
        );

        assert_eq!(
            grouped.left,
            vec![ModuleDef::Group(vec![
                ModuleName::Clock,
                ModuleName::Battery,
                ModuleName::Tray
            ])]
        );
    }

    #[test]
    fn an_island_splits_back_into_standalone_entries() {
        let modules = layout(vec![ModuleDef::Group(vec![
            ModuleName::Clock,
            ModuleName::Battery,
        ])]);

        let split = apply(
            &modules,
            &LayoutEdit::Ungroup {
                section: Section::Left,
                index:   0
            }
        );

        assert_eq!(
            split.left,
            vec![single(ModuleName::Clock), single(ModuleName::Battery)]
        );
    }

    #[test]
    fn ungrouping_a_standalone_entry_changes_nothing() {
        let modules = layout(vec![single(ModuleName::Clock)]);

        let split = apply(
            &modules,
            &LayoutEdit::Ungroup {
                section: Section::Left,
                index:   0
            }
        );

        assert_eq!(split.left, modules.left);
    }

    #[test]
    fn a_removed_entry_leaves_the_bar() {
        let modules = layout(vec![single(ModuleName::Clock), single(ModuleName::Battery)]);

        let removed = apply(
            &modules,
            &LayoutEdit::Remove {
                section: Section::Left,
                index:   0
            }
        );

        assert_eq!(removed.left, vec![single(ModuleName::Battery)]);
    }

    #[test]
    fn an_added_module_lands_at_the_end_of_its_section() {
        let modules = layout(vec![single(ModuleName::Clock)]);

        let added = apply(
            &modules,
            &LayoutEdit::Add {
                section: Section::Left,
                module:  ModuleName::Tray
            }
        );

        assert_eq!(
            added.left,
            vec![single(ModuleName::Clock), single(ModuleName::Tray)]
        );
    }

    #[test]
    fn the_available_list_skips_what_is_already_placed() {
        let modules = Modules {
            left:   vec![single(ModuleName::Clock)],
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
    fn a_placed_custom_module_is_not_offered_twice() {
        let modules = layout(vec![single(ModuleName::Custom("power".to_owned()))]);

        let available = available(&modules, &["power".to_owned()]);

        assert!(!available.contains(&ModuleName::Custom("power".to_owned())));
    }
}
