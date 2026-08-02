//! Writing an edited module layout back into the configuration file.
//!
//! The layout is stored as the file spells it — one list per section,
//! islands as lists inside it — and the pick of the editor follows the
//! module its edit moved.

use std::path::Path;

use hydebar_proto::config::{ModuleDef, Modules};
use log::warn;

use super::{LayoutEdit, SettingValue, Slot, writer};

/// Keeps the pick on the module the edit acted on.
///
/// A module that moved would otherwise leave the pick pointing at whatever
/// took its place, and the next button press would act on the wrong module.
pub(super) fn follow(edit: &LayoutEdit, modules: &Modules) -> Option<Slot> {
    let slot = edit.slot()?;

    match edit {
        LayoutEdit::Remove(_) => None,
        LayoutEdit::MoveEarlier(_) => Some(Slot {
            section: slot.section,
            index:   slot.index.saturating_sub(1)
        }),
        LayoutEdit::MoveLater(_) => Some(Slot {
            section: slot.section,
            index:   slot.index + 1
        }),
        LayoutEdit::MoveToPreviousSection(_) => slot.section.before().map(|section| Slot {
            section,
            index: section.entries(modules).len().saturating_sub(1)
        }),
        LayoutEdit::MoveToNextSection(_) => slot.section.after().map(|section| Slot {
            section,
            index: 0
        }),
        _ => Some(slot)
    }
}

/// Renders a bar entry as the value the configuration stores.
fn entry_value(entry: &ModuleDef) -> SettingValue {
    match entry {
        ModuleDef::Single(name) => SettingValue::Text(name.as_str().to_owned()),
        ModuleDef::Group(group) => SettingValue::List(
            group
                .iter()
                .map(|name| SettingValue::Text(name.as_str().to_owned()))
                .collect()
        )
    }
}

/// Renders a section as the list the configuration stores.
fn section_value(entries: &[ModuleDef]) -> SettingValue {
    SettingValue::List(entries.iter().map(entry_value).collect())
}

/// Writes every section of `modules` into the configuration file.
///
/// In one write on purpose: the file is watched, and three writes in a row
/// would reload the whole bar up to three times for one edit.
pub(super) fn store_layout(config_path: &Path, modules: &Modules) {
    let settings = vec![
        (["modules", "left"].as_slice(), section_value(&modules.left)),
        (
            ["modules", "center"].as_slice(),
            section_value(&modules.center)
        ),
        (
            ["modules", "right"].as_slice(),
            section_value(&modules.right)
        ),
    ];

    if let Err(err) = writer::write_settings(config_path, settings) {
        warn!("failed to store the module layout: {err}");
    }
}
