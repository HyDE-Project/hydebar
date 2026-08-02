//! Applying one edit of the module editor to a layout.

use hydebar_proto::config::Modules;

use super::{Entry, LayoutEdit};

/// Takes the module out of its section, leaving the rest joined sensibly.
///
/// A module that led an island hands the island over to the module after
/// it, so removing the head of a group does not silently dissolve the
/// group.
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_proto::config::{ModuleDef, ModuleName};

    use super::{
        super::{Section, Slot},
        *
    };

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
}
